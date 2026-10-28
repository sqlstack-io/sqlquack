//! A form's accepted combinations, and the combinations a recorded shape covers.

use std::collections::BTreeMap;

use sqlquack_core::provenance::models::Shape;

use crate::coverage::constants::{ANY_POSITION, LITERAL_KIND, NO_FACET, VALUE_KIND};
use crate::coverage::models::{ArgumentDomain, Form};

/// Every combination `form` accepts, rendered.
pub(crate) fn combinations(form: &Form) -> Vec<String> {
    let mut declared: Vec<Vec<String>> = Vec::new();
    for argument in form.arguments.iter().flatten() {
        declared.push(position_values(argument));
    }
    let arguments: Vec<Vec<String>> = match form.one_wise {
        true => alone(&declared),
        false => crossed(&declared),
    };
    let mut rendered: Vec<String> = Vec::new();
    for (dimension, values) in &form.each {
        for value in values {
            rendered.push(each_value(&form.construct, dimension, value));
        }
    }
    let facets = facet_choices(form);
    for construct in constructs(form) {
        for positions in &arguments {
            for chosen in &facets {
                rendered.push(render((form, construct), positions, chosen));
            }
        }
    }
    rendered
}

/// The combinations of `form` that `shape` covers, or `None` when the form does not accept it.
pub(crate) fn shape_combinations(form: &Form, shape: &Shape) -> Option<Vec<String>> {
    let construct = constructs(form).find(|construct| **construct == shape.construct)?;
    let positions = shape_positions(form, shape)?;
    let undeclared = shape.facets.keys().any(|dimension| {
        !form.facets.contains_key(dimension) && !form.each.contains_key(dimension)
    });
    if undeclared {
        return None;
    }
    let mut chosen: BTreeMap<&str, &str> = BTreeMap::new();
    for (dimension, values) in &form.facets {
        let value = facet(shape, dimension);
        if !values.iter().any(|accepted| accepted == value) {
            return None;
        }
        chosen.insert(dimension.as_str(), value);
    }
    let mut covered: Vec<String> = Vec::new();
    for (dimension, values) in &form.each {
        let value = facet(shape, dimension);
        if !values.iter().any(|accepted| accepted == value) {
            return None;
        }
        covered.push(each_value(&form.construct, dimension, value));
    }
    let arguments = match form.one_wise {
        true => alone(
            &positions
                .into_iter()
                .map(|value| vec![value])
                .collect::<Vec<_>>(),
        ),
        false => vec![positions],
    };
    for positions in &arguments {
        covered.push(render((form, construct), positions, &chosen));
    }
    Some(covered)
}

/// A shape as text, for shapes no declared form accepts.
pub(crate) fn render_shape(shape: &Shape) -> String {
    let mut arguments: Vec<String> = Vec::new();
    for argument in &shape.arguments {
        let kind = match argument.literal {
            true => LITERAL_KIND,
            false => VALUE_KIND,
        };
        arguments.push(position(&argument.family, Some(kind)));
    }
    let mut facets = String::new();
    for (dimension, value) in &shape.facets {
        facets.push_str(&format!(" {dimension}={value}"));
    }
    format!("{}({}){facets}", shape.construct, arguments.join(", "))
}

/// The declared positions `shape`'s arguments take, or `None` when one is not accepted.
fn shape_positions(form: &Form, shape: &Shape) -> Option<Vec<String>> {
    let Some(declared) = &form.arguments else {
        return Some(Vec::new());
    };
    let count_fits = match form.repeat {
        true => !declared.is_empty() && shape.arguments.len() >= declared.len(),
        false => shape.arguments.len() == declared.len(),
    };
    if !count_fits {
        return None;
    }
    let mut positions: Vec<String> = Vec::new();
    for (index, argument) in shape.arguments.iter().enumerate() {
        let domain = declared.get(index).or_else(|| declared.last())?;
        let kind = match argument.literal {
            true => LITERAL_KIND,
            false => VALUE_KIND,
        };
        let kind_fits = domain
            .kinds
            .as_ref()
            .is_none_or(|kinds| kinds.iter().any(|accepted| accepted == kind));
        if !domain.families.contains(&argument.family) || !kind_fits {
            return None;
        }
        if index < declared.len() {
            positions.push(position(
                &argument.family,
                domain.kinds.as_ref().map(|_| kind),
            ));
        }
    }
    Some(positions)
}

/// Each family and declared kind a position accepts, rendered.
fn position_values(argument: &ArgumentDomain) -> Vec<String> {
    let kinds: Vec<Option<&str>> = match &argument.kinds {
        Some(kinds) => kinds.iter().map(|kind| Some(kind.as_str())).collect(),
        None => vec![None],
    };
    let mut values: Vec<String> = Vec::new();
    for family in &argument.families {
        for kind in &kinds {
            values.push(position(family, *kind));
        }
    }
    values
}

/// Every choice of one value per position.
fn crossed(declared: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut prefixes: Vec<Vec<String>> = vec![Vec::new()];
    for values in declared {
        let mut next: Vec<Vec<String>> = Vec::new();
        for prefix in &prefixes {
            for value in values {
                let mut extended = prefix.clone();
                extended.push(value.clone());
                next.push(extended);
            }
        }
        prefixes = next;
    }
    prefixes
}

/// Each position's values alone, the other positions any value: one-wise rather than crossed.
fn alone(declared: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut rendered: Vec<Vec<String>> = Vec::new();
    for (index, values) in declared.iter().enumerate() {
        for value in values {
            let mut positions: Vec<String> = vec![ANY_POSITION.to_owned(); declared.len()];
            positions[index] = value.clone();
            rendered.push(positions);
        }
    }
    rendered
}

/// Every choice of one value per declared facet.
fn facet_choices(form: &Form) -> Vec<BTreeMap<&str, &str>> {
    let mut choices: Vec<BTreeMap<&str, &str>> = vec![BTreeMap::new()];
    for (dimension, values) in &form.facets {
        let mut next: Vec<BTreeMap<&str, &str>> = Vec::new();
        for prefix in &choices {
            for value in values {
                let mut extended = prefix.clone();
                extended.insert(dimension.as_str(), value.as_str());
                next.push(extended);
            }
        }
        choices = next;
    }
    choices
}

fn facet<'a>(shape: &'a Shape, dimension: &str) -> &'a str {
    shape.facets.get(dimension).map_or(NO_FACET, String::as_str)
}

fn each_value(construct: &str, dimension: &str, value: &str) -> String {
    format!("{construct} each {dimension}={value}")
}

/// The form's constructs: its own, then those it accepts alike.
fn constructs(form: &Form) -> impl Iterator<Item = &String> {
    std::iter::once(&form.construct).chain(&form.also)
}

fn position(family: &str, kind: Option<&str>) -> String {
    match kind {
        Some(kind) => format!("{family}:{kind}"),
        None => family.to_owned(),
    }
}

fn render(
    (form, construct): (&Form, &str),
    positions: &[String],
    facets: &BTreeMap<&str, &str>,
) -> String {
    let arguments = match &form.arguments {
        Some(_) => format!("({})", positions.join(", ")),
        None => String::new(),
    };
    let mut rendered_facets = String::new();
    for (dimension, value) in facets {
        rendered_facets.push_str(&format!(" {dimension}={value}"));
    }
    format!("{construct}{arguments}{rendered_facets}")
}
