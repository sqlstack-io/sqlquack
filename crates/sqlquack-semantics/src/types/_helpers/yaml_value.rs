//! A YAML document as a JSON value, for reading source files.

use std::collections::HashMap;

use saphyr_parser::{Event, Parser, ScalarStyle, StrInput};
use serde_json::{Map, Number, Value};

use crate::types::constants::MERGE_KEY;

/// The first document of `text` as a JSON value: plain scalars typed as YAML 1.2's core schema does.
pub(crate) fn yaml_value(text: &str) -> Result<Value, String> {
    let mut reader = Reader {
        parser: Parser::new_from_str(text),
        anchors: HashMap::new(),
    };
    loop {
        match reader.next()? {
            Event::StreamStart | Event::DocumentStart(_) | Event::Nothing => {}
            Event::StreamEnd => return Ok(Value::Null),
            event => return reader.node(event),
        }
    }
}

/// The parser and the anchored values seen so far.
struct Reader<'input> {
    parser: Parser<'input, StrInput<'input>>,
    anchors: HashMap<usize, Value>,
}

impl<'input> Reader<'input> {
    fn next(&mut self) -> Result<Event<'input>, String> {
        match self.parser.next_event() {
            Some(Ok((event, _))) => Ok(event),
            Some(Err(error)) => Err(format!(
                "line {} column {}: {}",
                error.marker().line(),
                error.marker().col() + 1,
                error.info()
            )),
            None => Err("the YAML ends unexpectedly".to_owned()),
        }
    }

    /// The value `event` starts.
    fn node(&mut self, event: Event<'input>) -> Result<Value, String> {
        let (value, anchor) = match event {
            Event::Alias(anchor) => {
                return self
                    .anchors
                    .get(&anchor)
                    .cloned()
                    .ok_or_else(|| "an alias names no anchor".to_owned());
            }
            Event::Scalar(text, style, anchor, _) => (scalar(&text, style), anchor),
            Event::SequenceStart(anchor, _) => {
                let mut items: Vec<Value> = Vec::new();
                loop {
                    match self.next()? {
                        Event::SequenceEnd => break,
                        event => items.push(self.node(event)?),
                    }
                }
                (Value::Array(items), anchor)
            }
            Event::MappingStart(anchor, _) => {
                let mut entries: Map<String, Value> = Map::new();
                let mut merged: Vec<Value> = Vec::new();
                loop {
                    let key = match self.next()? {
                        Event::MappingEnd => break,
                        event => self.node(event)?,
                    };
                    let key = match key {
                        Value::String(text) => text,
                        other => other.to_string(),
                    };
                    let event = self.next()?;
                    let value = self.node(event)?;
                    match key.as_str() {
                        MERGE_KEY => merged.push(value),
                        _ => {
                            entries.insert(key, value);
                        }
                    }
                }
                (Value::Object(with_merged(entries, merged)?), anchor)
            }
            _ => return Err("the YAML holds an unexpected event".to_owned()),
        };
        if anchor != 0 {
            self.anchors.insert(anchor, value.clone());
        }
        Ok(value)
    }
}

/// A scalar: quoted and block text as written, plain text typed as null, a boolean or a number.
fn scalar(text: &str, style: ScalarStyle) -> Value {
    if style != ScalarStyle::Plain {
        return Value::String(text.to_owned());
    }
    match text {
        "" | "~" | "null" | "Null" | "NULL" => Value::Null,
        "true" | "True" | "TRUE" => Value::Bool(true),
        "false" | "False" | "FALSE" => Value::Bool(false),
        _ => number(text).map_or_else(|| Value::String(text.to_owned()), Value::Number),
    }
}

/// `text` as an integer or a finite float, or None when it is neither.
fn number(text: &str) -> Option<Number> {
    match (text.parse::<i64>(), text.parse::<f64>()) {
        (Ok(integer), _) => Some(Number::from(integer)),
        (Err(_not_integer), Ok(float)) => Number::from_f64(float),
        (Err(_not_integer), Err(_not_float)) => None,
    }
}

/// `entries` with the unset keys of each mapping `<<` names, the first winning, as PyYAML merges.
fn with_merged(
    mut entries: Map<String, Value>,
    merged: Vec<Value>,
) -> Result<Map<String, Value>, String> {
    for value in merged {
        let mappings = match value {
            Value::Array(items) => items,
            mapping => vec![mapping],
        };
        for mapping in mappings {
            let Value::Object(mapping) = mapping else {
                return Err("a `<<` merge key must name a mapping or a list of them".to_owned());
            };
            for (key, value) in mapping {
                entries.entry(key).or_insert(value);
            }
        }
    }
    Ok(entries)
}
