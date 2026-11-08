//! What the parse tree says about the tokens: which name identifiers or aliases, which are binary operators.

use serde_json::Value;
use sqlquack_syntax::Expression;

use crate::format::_helpers::lexemes::Lexeme;
use crate::format::constants::BINARY_OPERATOR_NODES;

/// The fields of a serialised identifier node.
const IDENTIFIER_FIELDS: [&str; 4] = ["name", "quoted", "trailing_comments", "span"];
/// The field of a serialised node that names its alias, and of a select node.
const ALIAS_FIELD: &str = "alias";
const ALIASED_FIELD: &str = "this";
const SELECT_FIELD: &str = "select";
const PROJECTION_FIELD: &str = "expressions";
const SPAN_FIELD: &str = "span";
const LEFT_FIELD: &str = "left";
const RIGHT_FIELD: &str = "right";
const COLUMN_ALIASES_FIELD: &str = "column_aliases";
/// The field of a serialised UPDATE that holds its `column = value` pairs.
const SET_FIELD: &str = "set";

/// Per-token facts from the parse tree, and how many select lists it has.
pub(crate) struct ParsedNames {
    /// Tokens inside the source range of an identifier.
    pub(crate) identifiers: Vec<bool>,
    /// Tokens inside the source range of a name given with an alias, such as `b` in `a AS b`.
    pub(crate) aliases: Vec<bool>,
    /// Tokens the parse tree reads as the operator of a binary comparison, arithmetic or concatenation.
    pub(crate) binary_operators: Vec<bool>,
    /// Words matching a name the parse tree records without a span, which therefore keep their case.
    pub(crate) unspanned: Vec<bool>,
    /// The number of select lists in the parse tree.
    pub(crate) selects: usize,
}

/// A binary operator node: its spellings, and its range and where its operands end and start when recorded.
struct Binary {
    spellings: &'static [&'static str],
    span: Option<(usize, usize)>,
    left_end: Option<usize>,
    right_start: Option<usize>,
}

/// The ranges the parse tree records: every identifier and every alias name, in characters.
#[derive(Default)]
struct Ranges {
    identifiers: Vec<(usize, usize)>,
    aliases: Vec<(usize, usize)>,
    /// Every recorded node range.
    spans: Vec<(usize, usize)>,
    binaries: Vec<Binary>,
    /// Alias names recorded as bare strings, without a span.
    unspanned: Vec<String>,
    selects: usize,
}

/// Mark the tokens of `lexemes` the parse trees of `statements` name identifiers or aliases.
pub(crate) fn parsed_names(
    sql: &str,
    statements: &[Expression],
    lexemes: &[Lexeme],
) -> Result<ParsedNames, String> {
    let bytes: Vec<usize> = sql
        .char_indices()
        .map(|(byte, _)| byte)
        .chain([sql.len()])
        .collect();
    let byte = |offset: usize| bytes.get(offset).copied().unwrap_or(sql.len());
    let trees: Vec<Value> = statements
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())?;
    let ranges = trees
        .iter()
        .map(collect)
        .fold(Ranges::default(), Ranges::merged);
    let to_bytes = |ranges: &[(usize, usize)]| -> Vec<(usize, usize)> {
        let mut converted: Vec<(usize, usize)> = ranges
            .iter()
            .map(|&(start, end)| (byte(start), byte(end)))
            .collect();
        converted.sort_unstable();
        converted
    };
    let (identifiers, aliases) = (
        Covering::new(to_bytes(&ranges.identifiers)),
        Covering::new(to_bytes(&ranges.aliases)),
    );
    Ok(ParsedNames {
        identifiers: lexemes
            .iter()
            .map(|lexeme| identifiers.covers(lexeme))
            .collect(),
        aliases: lexemes
            .iter()
            .map(|lexeme| aliases.covers(lexeme))
            .collect(),
        binary_operators: binary_operators(&ranges, lexemes, &byte),
        unspanned: lexemes
            .iter()
            .map(|lexeme| names_word(&ranges.unspanned, lexeme))
            .collect(),
        selects: ranges.selects,
    })
}

/// For every token, whether a binary operator node of the parse tree has it as its operator.
fn binary_operators(
    ranges: &Ranges,
    lexemes: &[Lexeme],
    byte: &dyn Fn(usize) -> usize,
) -> Vec<bool> {
    let mut operators: Vec<bool> = vec![false; lexemes.len()];
    let spans: Vec<(usize, usize)> = ranges
        .spans
        .iter()
        .map(|&(start, end)| (byte(start), byte(end)))
        .collect();
    for binary in &ranges.binaries {
        let (start, end) = binary
            .span
            .map_or((0, usize::MAX), |(start, end)| (byte(start), byte(end)));
        let first = lexemes.partition_point(|lexeme| lexeme.start < start);
        let last = lexemes.partition_point(|lexeme| lexeme.end <= end);
        let inside: Vec<usize> = (first..last.max(first)).collect();
        let found = match (binary.left_end.map(byte), binary.right_start.map(byte)) {
            (Some(left_end), _) => inside
                .iter()
                .copied()
                .find(|&index| lexemes[index].start >= left_end),
            (None, Some(right_start)) => inside
                .iter()
                .copied()
                .rfind(|&index| lexemes[index].end <= right_start),
            (None, None) if binary.span.is_some() => {
                let candidates: Vec<usize> = inside
                    .iter()
                    .copied()
                    .filter(|&index| {
                        binary.spellings.contains(&lexemes[index].raw.as_str())
                            && !in_inner_node(&spans, (start, end), &lexemes[index])
                    })
                    .collect();
                match candidates.as_slice() {
                    [only] => Some(*only),
                    _ => None,
                }
            }
            (None, None) => None,
        };
        if let Some(index) =
            found.filter(|&index| binary.spellings.contains(&lexemes[index].raw.as_str()))
        {
            operators[index] = true;
        }
    }
    operators
}

/// Sorted byte ranges, with the furthest end among each range and those before it.
struct Covering {
    ranges: Vec<(usize, usize)>,
    furthest_ends: Vec<usize>,
}

impl Covering {
    fn new(ranges: Vec<(usize, usize)>) -> Self {
        let furthest_ends: Vec<usize> = ranges
            .iter()
            .scan(0, |furthest, &(_, end)| {
                *furthest = (*furthest).max(end);
                Some(*furthest)
            })
            .collect();
        Self {
            ranges,
            furthest_ends,
        }
    }

    /// Whether one of the ranges covers `lexeme`.
    fn covers(&self, lexeme: &Lexeme) -> bool {
        let before = self
            .ranges
            .partition_point(|&(start, _)| start <= lexeme.start);
        before
            .checked_sub(1)
            .is_some_and(|last| self.furthest_ends[last] >= lexeme.end)
    }
}

/// The identifier and alias ranges and the select lists of `value` and every node under it, walked without recursion so a deep tree cannot exhaust the stack.
fn collect(value: &Value) -> Ranges {
    let mut ranges = Ranges::default();
    let mut pending: Vec<&Value> = vec![value];
    while let Some(node) = pending.pop() {
        match node {
            Value::Object(fields) => pending.extend(fields.values()),
            Value::Array(items) => pending.extend(items),
            _ => {}
        }
        ranges = ranges.merged(own_ranges(node));
    }
    ranges
}

/// The identifier and alias ranges and the select list `value` itself records, without its children.
fn own_ranges(value: &Value) -> Ranges {
    Ranges {
        identifiers: identifier_range(value).into_iter().collect(),
        aliases: value
            .get(ALIAS_FIELD)
            .filter(|node| node.get(ALIASED_FIELD).is_some())
            .and_then(|node| node.get(ALIAS_FIELD))
            .and_then(identifier_range)
            .into_iter()
            .collect(),
        spans: value
            .get(SPAN_FIELD)
            .and_then(|span| Some((span_offset(span, "start")?, span_offset(span, "end")?)))
            .into_iter()
            .collect(),
        binaries: binary(value)
            .into_iter()
            .chain(assignments(value))
            .collect(),
        unspanned: unspanned_names(value),
        selects: usize::from(
            value
                .get(SELECT_FIELD)
                .is_some_and(|node| node.get(PROJECTION_FIELD).is_some()),
        ),
    }
}

impl Ranges {
    /// Both sets of ranges and select lists together.
    fn merged(mut self, other: Ranges) -> Ranges {
        self.identifiers.extend(other.identifiers);
        self.aliases.extend(other.aliases);
        self.spans.extend(other.spans);
        self.binaries.extend(other.binaries);
        self.unspanned.extend(other.unspanned);
        self.selects += other.selects;
        self
    }
}

/// Whether `lexeme` lies in a recorded node range strictly inside the range `outer`.
fn in_inner_node(spans: &[(usize, usize)], outer: (usize, usize), lexeme: &Lexeme) -> bool {
    spans.iter().any(|&(start, end)| {
        (start, end) != outer
            && start >= outer.0
            && end <= outer.1
            && start <= lexeme.start
            && lexeme.end <= end
    })
}

/// Whether `lexeme` spells one of `names`, ignoring ASCII case.
fn names_word(names: &[String], lexeme: &Lexeme) -> bool {
    names
        .iter()
        .any(|name| name.eq_ignore_ascii_case(&lexeme.raw))
}

/// The alias and column alias names `value` records as bare strings, which carry no span.
fn unspanned_names(value: &Value) -> Vec<String> {
    let alias = value.get(ALIAS_FIELD).and_then(Value::as_str);
    let column_aliases = value
        .get(COLUMN_ALIASES_FIELD)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    alias
        .into_iter()
        .chain(column_aliases)
        .map(str::to_string)
        .collect()
}

/// The binary operator node `value` is, when it is one with a recorded span.
fn binary(value: &Value) -> Option<Binary> {
    let fields = value.as_object().filter(|fields| fields.len() == 1)?;
    let (kind, node) = fields.iter().next()?;
    let (_, spellings) = BINARY_OPERATOR_NODES
        .iter()
        .find(|(name, _)| name == kind)?;
    let operand_span = |field: &str| -> Option<&Value> {
        node.get(field)?
            .as_object()?
            .values()
            .next()?
            .get(SPAN_FIELD)
    };
    Some(Binary {
        spellings,
        span: node
            .get(SPAN_FIELD)
            .and_then(|span| Some((span_offset(span, "start")?, span_offset(span, "end")?))),
        left_end: operand_span(LEFT_FIELD).and_then(|span| span_offset(span, "end")),
        right_start: operand_span(RIGHT_FIELD).and_then(|span| span_offset(span, "start")),
    })
}

/// The `=` of each `column = value` pair of an UPDATE's SET, which the tree keeps as a pair, not a comparison.
fn assignments(value: &Value) -> Vec<Binary> {
    let pairs = value.get(SET_FIELD).and_then(Value::as_array);
    pairs
        .into_iter()
        .flatten()
        .filter_map(|pair| identifier_range(pair.as_array()?.first()?))
        .map(|(_, end)| Binary {
            spellings: &["="],
            span: None,
            left_end: Some(end),
            right_start: None,
        })
        .collect()
}

/// The character range of `value` when it is an identifier node with a recorded span.
fn identifier_range(value: &Value) -> Option<(usize, usize)> {
    let fields = value.as_object()?;
    let identifier = fields.get("name").is_some_and(Value::is_string)
        && fields.get("quoted").is_some_and(Value::is_boolean)
        && fields
            .keys()
            .all(|key| IDENTIFIER_FIELDS.contains(&key.as_str()));
    let span = fields.get("span").filter(|_| identifier)?;
    Some((span_offset(span, "start")?, span_offset(span, "end")?))
}

/// One character offset of a serialised span.
fn span_offset(span: &Value, key: &str) -> Option<usize> {
    span.get(key)?.as_u64().map(|offset| offset as usize)
}
