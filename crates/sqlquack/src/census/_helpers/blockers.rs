//! Traces each refused file to the refusals at the root of the models it reads.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_rules::conversion::models::Schema;

use crate::census::_helpers::corpus::model_relation;
use crate::census::_helpers::inspect::FileCensus;
use crate::census::constants::{
    CYCLE_ROOT, MODEL_ROOT_PREFIX, RELATION_ROOT_PREFIX, UNKNOWN_RELATION_ROOT, UPSTREAM_MODEL_ROOT,
};
use crate::census::models::{BlockedFile, BlockerReport, RootBlocker, RootKey, UpstreamBlock};

/// What the census's refused files are blocked by, through the model graph of `files`.
pub(crate) fn blockers(
    files: &[PathBuf],
    censuses: &[FileCensus],
    schema: &Schema,
    names: bool,
) -> BlockerReport {
    let graph = Graph::new(files, censuses, schema);
    let (mut own, mut roots) = graph.roots();
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
        });
    }
    BlockerReport {
        single_root_files: blocked.iter().filter(|file| file.roots.len() == 1).count(),
        roots: ranking(&blocked),
        files: blocked,
    }
}

/// The model graph: each file's own root refusals, and the refused models it reads.
struct Graph {
    models: Vec<String>,
    refused: Vec<bool>,
    own: Vec<BTreeSet<RootKey>>,
    upstream: Vec<BTreeSet<usize>>,
    unknown: Vec<BTreeSet<String>>,
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
        };
        for (index, census) in censuses.iter().enumerate() {
            let model_of = |relation: &String| by_relation.get(&folded(relation)).copied();
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

fn model_name(path: &std::path::Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

/// Each root with the files it refuses directly, downstream and alone, most files first.
fn ranking(files: &[BlockedFile]) -> Vec<RootBlocker> {
    let mut counts: BTreeMap<&RootKey, (usize, usize, usize)> = BTreeMap::new();
    for file in files {
        for root in &file.roots {
            let entry = counts.entry(root).or_default();
            entry.1 += 1;
            if file.own.contains(root) {
                entry.0 += 1;
            }
            if file.roots.len() == 1 {
                entry.2 += 1;
            }
        }
    }
    let mut ranked: Vec<RootBlocker> = counts
        .into_iter()
        .map(|(key, (direct, total, sole))| RootBlocker {
            category: key.category.clone(),
            construct: key.construct.clone(),
            direct,
            downstream: total - direct,
            total,
            sole,
        })
        .collect();
    ranked.sort_by_key(|root| {
        (
            Reverse(root.total),
            Reverse(root.direct),
            root.category.clone(),
            root.construct.clone(),
        )
    });
    ranked
}
