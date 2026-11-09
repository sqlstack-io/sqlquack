//! Shared helpers of the SQL Server parser tests.

use serde_json::Value;

use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::ExpectedVerdict;

/// The opening and closing text of each nesting form, cycled through by `alternating`.
const NESTING_FORMS: [(&str, &str); 7] = [
    ("(", ")"),
    ("(SELECT ", ")"),
    ("CASE WHEN 1 = 1 THEN ", " END"),
    ("ABS(", ")"),
    ("CAST(", " AS int)"),
    ("-(", ")"),
    ("IIF(1 = 1, ", ", 0)"),
];

/// The parser's verdict on `sql`.
pub(crate) fn verdict(sql: &str) -> ExpectedVerdict {
    parse_strict(sql)
        .map(|parsed| {
            parsed.gaps.first().map_or(ExpectedVerdict::Tree, |gap| {
                ExpectedVerdict::Gap(gap.construct)
            })
        })
        .unwrap_or_else(|failure| ExpectedVerdict::Refused {
            syntax: failure.parse_error().syntax,
        })
}

/// Every identifier node in `tree` as its name and its character range.
fn identifiers(tree: &Value) -> Vec<(String, Option<(usize, usize)>)> {
    let own = tree
        .as_object()
        .filter(|fields| fields.get("quoted").is_some_and(Value::is_boolean))
        .and_then(|fields| {
            let name: String = fields.get("name")?.as_str()?.to_owned();
            let span: Option<(usize, usize)> = fields.get("span").and_then(|span| {
                Some((
                    usize::try_from(span.get("start")?.as_u64()?).ok()?,
                    usize::try_from(span.get("end")?.as_u64()?).ok()?,
                ))
            });
            Some((name, span))
        });
    let children = tree
        .as_object()
        .into_iter()
        .flat_map(|fields| fields.values())
        .chain(tree.as_array().into_iter().flatten());
    own.into_iter()
        .chain(children.flat_map(identifiers))
        .collect()
}

/// Each one-part identifier of `sql`'s tree as its name and the undelimited text its span covers.
pub(crate) fn identifier_texts(sql: &str) -> Vec<(String, Option<String>)> {
    let parsed = parse_strict(sql).expect("the statement parses");
    assert!(parsed.gaps.is_empty(), "{:?}", parsed.gaps);
    let tree: Value = serde_json::to_value(&parsed.statements).expect("the tree serialises");
    identifiers(&tree)
        .into_iter()
        .filter(|(name, _)| !name.contains('.'))
        .map(|(name, span)| {
            let text: Option<String> = span.and_then(|(start, end)| {
                sql.get(start..end)
                    .map(|text| text.trim_matches(['[', ']']).to_owned())
            });
            (name, text)
        })
        .collect()
}

/// `head` followed by `links` copies of `link`: a flat chain the tree nests one link per level.
pub(crate) fn flat_chain(head: &str, link: &str, links: usize) -> String {
    format!("{head}{}", link.repeat(links))
}

/// `levels` chains of `links` links, each nested in the next one's first operand within parentheses.
pub(crate) fn nested_chains(operand: &str, link: &str, links: usize, levels: usize) -> String {
    let closing: String = format!("{})", link.repeat(links)).repeat(levels);
    format!("{}{operand}{closing}", "(".repeat(levels))
}

/// `SELECT` over `levels` nested forms, cycling through every recursive form.
pub(crate) fn alternating(levels: usize) -> String {
    let form = |level: usize| NESTING_FORMS[level % NESTING_FORMS.len()];
    let opening: String = (0..levels).map(|level| form(level).0).collect();
    let closing: String = (0..levels).rev().map(|level| form(level).1).collect();
    format!("SELECT {opening}1{closing}")
}

/// The gaps `sql` parses with on a 2 MB thread, its tree serialized there too, or its error message.
pub(crate) fn gaps_on_small_stack(sql: String) -> Result<Vec<&'static str>, String> {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            parse_strict(&sql)
                .map(|parsed| {
                    serde_json::to_string(&parsed.statements).expect("the tree serialises");
                    parsed.gaps.iter().map(|gap| gap.construct).collect()
                })
                .map_err(|failure| failure.parse_error().message.clone())
        })
        .expect("the parse thread starts")
        .join()
        .expect("the parse thread finishes")
}

/// Each recorded statement whose verdict contradicts SQL Server's: a tree for a statement it rejects, or a refusal of one it accepts.
pub(crate) fn disagreements(receipts: &str) -> Vec<String> {
    receipts
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|receipt| {
            let sql: &str = receipt["statement"].as_str()?;
            let accepted: bool = receipt["outcome"] == "ok";
            let ours: ExpectedVerdict = verdict(sql);
            let gap: bool = matches!(ours, ExpectedVerdict::Gap(_));
            let refused: bool = matches!(ours, ExpectedVerdict::Refused { .. });
            let agrees: bool = gap || refused != accepted;
            (!agrees).then(|| format!("{sql}: SQL Server {}, parser {ours:?}", receipt["outcome"]))
        })
        .collect()
}
