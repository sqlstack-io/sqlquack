# sqlquack-syntax-sqlserver

A SQL Server 2022 Transact-SQL scanner, lossless tokenizer and parser that builds the shared
`sqlquack-syntax` tree. Each verdict it gives is checked against SQL Server 2022 itself, with
`SET PARSEONLY ON`.

## Entry points

- `scan`: the tokens the grammar reads. A lexical fault ends the list as a `Fault` token, which
  the parser reaches in turn.
- `tokenize`: every byte of the input as a token, blank run or comment, typed as the shared
  tokenizer types them. The formatter uses it.
- `keyword_status` and `is_reserved`: whether a word is a keyword and whether it is reserved,
  compared without case.
- `parse`: every statement of a batch, or the error SQL Server's parser reports. `parse_strict`
  is the same, but refuses any error.

This crate defines its own `Parsed`, `Gap` and `StatementRange` rather than re-exporting
`sqlquack_syntax`'s. `Parsed.ranges` lists where each statement was written, in source order,
including statements nested in `BEGIN ... END`, `IF`, `WHILE`, `TRY ... CATCH` and procedure
bodies, each with its nesting depth. The formatter lays out Transact-SQL by these ranges.

## Gaps

A statement holding a construct the shared tree cannot hold parses as a `Command` that keeps its
text, and `Parsed.gaps` names the construct and its byte range, such as `cursor`,
`xml_method` or `throw`. A gap is not a refusal: SQL Server accepts the statement, and the tree
just does not model it.

## Checks

`src/parse/tests/test_parse.rs` holds the verdict rows: each input with what SQL Server 2022 does
with it under `PARSEONLY`, a tree or a refusal, with the error number in the row's description.
When the parser starts to accept a new form, record SQL Server's verdict for it and add a row,
and add a row for each near miss the engine refuses.
