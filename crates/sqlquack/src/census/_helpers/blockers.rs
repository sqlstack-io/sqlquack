//! Traces each refused file to the refusals at the root of the models it reads.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use sqlquack_diagnostics::refusals::models::RefusalCategory;
use sqlquack_semantics::snowflake::typing::types::Schema;

use crate::census::_helpers::combinations::combinations;
use crate::census::_helpers::corpus::{model_relation, referenced_model};
use crate::census::_helpers::files::model_name;
use crate::census::_helpers::inspect::FileCensus;
use crate::census::constants::{
    CYCLE_ROOT, MODEL_ROOT_PREFIX, RELATION_ROOT_PREFIX, UNKNOWN_RELATION_ROOT,
    UPSTREAM_MISSING_LABEL, UPSTREAM_MODEL_ROOT, UPSTREAM_REFUSED_LABEL,
};
use crate::census::models::{
    BlockedFile, BlockerReport, ModelReads, RootBlocker, RootKey, UpstreamBlock,
    UpstreamRefusedFile,
};
use crate::planning::models::PlanInput;

/// What blocks the refused files, the model graph, and each converted file's refused upstream.
pub(crate) fn blockers(
    files: &[PathBuf],
    censuses: &[FileCensus],
    schema: &Schema,
    names: bool,
) -> (BlockerReport, PlanInput, Vec<BTreeSet<usize>>) {
    let graph = Graph::new(files, censuses, schema);
    let (mut own, mut roots) = graph.roots();
    let incomplete = graph.incomplete();
    let mut models = graph.models.clone();
    let mut order: Vec<usize> = (0..models.len()).collect();
    order.sort_by(|left, right| models[*left].cmp(&models[*right]));
    if !names {
        for (number, index) in order.iter().enumerate() {
            models[*index] = format!("#{}", number + 1);
        }
        for keys in roots.iter_mut().chain(own.iter_mut()) {
            *keys = std::mem::take(keys).into_iter().map(unnamed).collect();
        }
    }
    let ModelReads {
        reads,
        needs,
        missing,
    } = model_reads(files, censuses, schema);
    let refused_upstream = upstream_refused(censuses, &reads);
    let plan_input = PlanInput {
        own: own.clone(),
        needs,
        reads: reads.clone(),
        parsed: censuses.iter().map(|census| census.parsed).collect(),
        incomplete: incomplete.clone(),
        missing: missing
            .iter()
            .map(|relations| !relations.is_empty())
            .collect(),
    };
    let mut upstream_refused: Vec<UpstreamRefusedFile> = Vec::new();
    for index in &order {
        let mut labels: Vec<String> = Vec::new();
        for upstream in &refused_upstream[*index] {
            labels.push(format!("{UPSTREAM_REFUSED_LABEL}{}", models[*upstream]));
        }
        if !graph.refused[*index] {
            let mut absent: BTreeSet<&String> = missing[*index].iter().collect();
            for upstream in upstream_reads(*index, &reads) {
                absent.extend(&missing[upstream]);
            }
            for relation in absent {
                let shown = match names {
                    true => relation.as_str(),
                    false => "",
                };
                labels.push(format!("{UPSTREAM_MISSING_LABEL}{shown}"));
            }
            labels.dedup();
        }
        if labels.is_empty() {
            continue;
        }
        upstream_refused.push(UpstreamRefusedFile {
            model: models[*index].clone(),
            labels,
        });
    }
    let mut blocked: Vec<BlockedFile> = Vec::new();
    for index in order.into_iter().filter(|index| graph.refused[*index]) {
        blocked.push(BlockedFile {
            model: models[index].clone(),
            own: own[index].iter().cloned().collect(),
            upstream: graph.upstream[index]
                .iter()
                .map(|upstream| UpstreamBlock {
                    model: models[*upstream].clone(),
                    roots: roots[*upstream].iter().cloned().collect(),
                })
                .collect(),
            unknown_relations: match names {
                true => graph.unknown[index].iter().cloned().collect(),
                false => Vec::new(),
            },
            roots: roots[index].iter().cloned().collect(),
            placed: censuses[index].placed.clone(),
            possibly_incomplete: incomplete[index],
        });
    }
    let report = BlockerReport {
        single_root_files: blocked
            .iter()
            .filter(|file| file.roots.len() == 1 && !file.possibly_incomplete)
            .count(),
        possibly_incomplete_files: blocked
            .iter()
            .filter(|file| file.possibly_incomplete)
            .count(),
        roots: ranking(&blocked),
        combinations: combinations(&blocked),
        files: blocked,
        upstream_refused,
    };
    (report, plan_input, refused_upstream)
}

/// The model graph: each file's own root refusals, and the refused models it reads.
struct Graph {
    models: Vec<String>,
    refused: Vec<bool>,
    own: Vec<BTreeSet<RootKey>>,
    upstream: Vec<BTreeSet<usize>>,
    unknown: Vec<BTreeSet<String>>,
    possibly_incomplete: Vec<bool>,
}

impl Graph {
    fn new(files: &[PathBuf], censuses: &[FileCensus], schema: &Schema) -> Self {
        let mut by_relation: BTreeMap<String, usize> = BTreeMap::new();
        for (index, file) in files.iter().enumerate() {
            if let Some(relation) = model_relation(file) {
                by_relation.entry(folded(&relation)).or_insert(index);
            }
        }
        let schema_relations: BTreeSet<String> = schema
            .relations
            .keys()
            .map(|relation| folded(relation))
            .collect();
        let refused: Vec<bool> = censuses
            .iter()
            .map(|census| !census.parsed || !census.refusal_categories.is_empty())
            .collect();
        let mut graph = Self {
            models: files.iter().map(|file| model_name(file)).collect(),
            refused: refused.clone(),
            own: Vec::with_capacity(censuses.len()),
            upstream: Vec::with_capacity(censuses.len()),
            unknown: Vec::with_capacity(censuses.len()),
            possibly_incomplete: censuses
                .iter()
                .map(|census| census.possibly_incomplete)
                .collect(),
        };
        for (index, census) in censuses.iter().enumerate() {
            let model_of =
                |relation: &String| by_relation.get(&referenced_model(relation)).copied();
            let upstream: BTreeSet<usize> = census
                .references
                .iter()
                .chain(&census.unknown_relations)
                .filter_map(model_of)
                .filter(|upstream| *upstream != index && refused[*upstream])
                .collect();
            let unknown: BTreeSet<String> = census
                .unknown_relations
                .iter()
                .filter(|relation| {
                    model_of(relation).is_none() && !schema_relations.contains(&folded(relation))
                })
                .cloned()
                .collect();
            let mut own: BTreeSet<RootKey> = census
                .root_constructs
                .iter()
                .map(|(category, construct)| root_key(*category, construct.clone()))
                .collect();
            for relation in &census.unknown_relations {
                own.extend(relation_root(relation, model_of(relation), &refused));
            }
            graph.own.push(own);
            graph.upstream.push(upstream);
            graph.unknown.push(unknown);
        }
        graph
    }

    /// Each file's own and inherited root refusals; models refused only by each other own a cycle.
    fn roots(&self) -> (Vec<BTreeSet<RootKey>>, Vec<BTreeSet<RootKey>>) {
        let roots = self.inherited(self.own.clone());
        let cyclic: Vec<usize> = (0..roots.len())
            .filter(|index| self.refused[*index] && roots[*index].is_empty())
            .filter(|index| self.reaches(*index, *index))
            .collect();
        if cyclic.is_empty() {
            return (self.own.clone(), roots);
        }
        let mut own = self.own.clone();
        for index in cyclic {
            own[index].insert(root_key(
                RefusalCategory::UnresolvedName,
                CYCLE_ROOT.to_owned(),
            ));
        }
        let roots = self.inherited(own.clone());
        (own, roots)
    }

    /// Whether each file, or a refused model upstream of it, may hold refusals not collected.
    fn incomplete(&self) -> Vec<bool> {
        (0..self.models.len())
            .map(|index| self.possibly_incomplete[index] || self.incomplete_upstream(index))
            .collect()
    }

    /// Whether a refused model upstream of `start` may hold refusals not collected.
    fn incomplete_upstream(&self, start: usize) -> bool {
        let mut upstream = self.upstream_of(start);
        upstream.any(|index| self.possibly_incomplete[index])
    }

    /// Every refused model upstream of `start`, each once.
    fn upstream_of(&self, start: usize) -> impl Iterator<Item = usize> {
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut pending: Vec<usize> = self.upstream[start].iter().copied().collect();
        while let Some(index) = pending.pop() {
            if seen.insert(index) {
                pending.extend(self.upstream[index].iter().copied());
            }
        }
        seen.into_iter()
    }

    /// Whether `target` is upstream of `start`, through refused models.
    fn reaches(&self, start: usize, target: usize) -> bool {
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut pending: Vec<usize> = self.upstream[start].iter().copied().collect();
        while let Some(index) = pending.pop() {
            if index == target {
                return true;
            }
            if seen.insert(index) {
                pending.extend(self.upstream[index].iter().copied());
            }
        }
        false
    }

    /// Each file's `own` roots and its refused upstream models', to a fixed point.
    fn inherited(&self, own: Vec<BTreeSet<RootKey>>) -> Vec<BTreeSet<RootKey>> {
        let mut roots = own;
        let mut changed = true;
        while changed {
            changed = false;
            for index in 0..roots.len() {
                let inherited: BTreeSet<RootKey> = self.upstream[index]
                    .iter()
                    .flat_map(|upstream| roots[*upstream].iter().cloned())
                    .filter(|root| !roots[index].contains(root))
                    .collect();
                changed |= !inherited.is_empty();
                roots[index].extend(inherited);
            }
        }
        roots
    }
}

/// The models each file reads, the untyped refused ones among them, and absent ones it references.
fn model_reads(files: &[PathBuf], censuses: &[FileCensus], schema: &Schema) -> ModelReads {
    let mut by_relation: BTreeMap<String, usize> = BTreeMap::new();
    let mut untyped: BTreeSet<usize> = BTreeSet::new();
    let typed_relations: BTreeSet<String> =
        schema.relations.keys().map(|known| folded(known)).collect();
    for (index, file) in files.iter().enumerate() {
        if let Some(relation) = model_relation(file) {
            let typed = typed_relations.contains(&folded(&relation));
            let census = &censuses[index];
            if !typed && (!census.parsed || !census.refusal_categories.is_empty()) {
                untyped.insert(index);
            }
            by_relation.entry(folded(&relation)).or_insert(index);
        }
    }
    let models_of = |index: usize, relations: &BTreeSet<String>| {
        let mut models: BTreeSet<usize> = BTreeSet::new();
        for relation in relations {
            if let Some(model) = by_relation.get(&referenced_model(relation))
                && *model != index
            {
                models.insert(*model);
            }
        }
        models
    };
    let mut reads: Vec<BTreeSet<usize>> = Vec::with_capacity(censuses.len());
    let mut needs: Vec<BTreeSet<usize>> = Vec::with_capacity(censuses.len());
    let mut missing: Vec<BTreeSet<String>> = Vec::with_capacity(censuses.len());
    for (index, census) in censuses.iter().enumerate() {
        let read = models_of(index, &census.references);
        needs.push(read.intersection(&untyped).copied().collect());
        reads.push(read);
        missing.push(
            census
                .model_references
                .iter()
                .filter(|relation| !by_relation.contains_key(&folded(relation)))
                .cloned()
                .collect(),
        );
    }
    ModelReads {
        reads,
        needs,
        missing,
    }
}

/// For each converted file, the refused models upstream of it; empty when it runs end to end.
fn upstream_refused(censuses: &[FileCensus], reads: &[BTreeSet<usize>]) -> Vec<BTreeSet<usize>> {
    let converted: Vec<bool> = censuses
        .iter()
        .map(|census| census.parsed && census.refusal_categories.is_empty())
        .collect();
    let mut refused_upstream: Vec<BTreeSet<usize>> = Vec::with_capacity(censuses.len());
    for (index, is_converted) in converted.iter().enumerate() {
        let mut refused: BTreeSet<usize> = BTreeSet::new();
        if *is_converted {
            refused.extend(
                upstream_reads(index, reads)
                    .into_iter()
                    .filter(|model| !converted[*model]),
            );
        }
        refused_upstream.push(refused);
    }
    refused_upstream
}

/// Every model upstream of `start` through the models each file reads, each once.
fn upstream_reads(start: usize, reads: &[BTreeSet<usize>]) -> BTreeSet<usize> {
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    let mut pending: Vec<usize> = reads[start].iter().copied().collect();
    while let Some(index) = pending.pop() {
        if index != start && seen.insert(index) {
            pending.extend(reads[index].iter().copied());
        }
    }
    seen
}

/// The root an unresolved relation stands for; none for a refused model, whose roots count instead.
fn relation_root(relation: &str, model: Option<usize>, refused: &[bool]) -> Option<RootKey> {
    match model {
        Some(model) if refused[model] => None,
        Some(_) => Some(root_key(
            RefusalCategory::UnresolvedName,
            format!("{MODEL_ROOT_PREFIX}{relation}"),
        )),
        None => Some(root_key(
            RefusalCategory::UnresolvedName,
            format!("{RELATION_ROOT_PREFIX}{relation}"),
        )),
    }
}

/// A root with any model or relation name folded away.
fn unnamed(root: RootKey) -> RootKey {
    let construct = if root.construct.starts_with(MODEL_ROOT_PREFIX) {
        UPSTREAM_MODEL_ROOT.to_owned()
    } else if root.construct.starts_with(RELATION_ROOT_PREFIX) {
        UNKNOWN_RELATION_ROOT.to_owned()
    } else {
        root.construct
    };
    RootKey { construct, ..root }
}

/// A relation name as Snowflake compares unquoted identifiers: in upper case.
fn folded(relation: &str) -> String {
    relation.to_ascii_uppercase()
}

fn root_key(category: RefusalCategory, construct: String) -> RootKey {
    RootKey {
        category: category.label().to_owned(),
        construct,
    }
}

/// Each root with the files it refuses directly, downstream and alone, most files first.
fn ranking(files: &[BlockedFile]) -> Vec<RootBlocker> {
    let mut counts: BTreeMap<&RootKey, [usize; 4]> = BTreeMap::new();
    for file in files {
        for root in &file.roots {
            let entry = counts.entry(root).or_default();
            entry[1] += 1;
            if file.own.contains(root) {
                entry[0] += 1;
            }
            if file.roots.len() == 1 {
                entry[if file.possibly_incomplete { 3 } else { 2 }] += 1;
            }
        }
    }
    let mut ranked: Vec<RootBlocker> = counts
        .into_iter()
        .map(|(key, [direct, total, sole, sole_uncertain])| RootBlocker {
            category: key.category.clone(),
            construct: key.construct.clone(),
            direct,
            downstream: total - direct,
            total,
            sole,
            sole_uncertain,
        })
        .collect();
    ranked.sort_by_key(|root| {
        (
            Reverse(root.sole),
            Reverse(root.total),
            Reverse(root.direct),
            root.category.clone(),
            root.construct.clone(),
        )
    });
    ranked
}
