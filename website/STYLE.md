# Docs style guide

These rules apply to every page under `website/src/content/docs/`. Pages that follow them well:
`cli/check.mdx`, `diagnostics/overview.mdx` and `policy.mdx`.

## Write for users

- Document what a user does, sees and must decide: commands, flags, config keys, defaults, limits,
  diagnostic codes and messages, output, exit codes, and the fix for each problem.
- Leave out how SQLQuack works inside: crate layout, internal tree formats, building blocks and
  internal structures. Include a mechanism only when the user must act
  on it, and then say what to do (for example, which flag forces a fresh check).
- Never document behaviour you haven't checked in the source or with the real `sqlquack` command.
  Paste real output, trimmed.

## Structure

- Lead with what the feature does and a short example: a command, a config block or the output a
  user sees.
- Organise sections around the user's tasks ("Declaring a schema", "Allowing a divergence"), not
  around the implementation.
- One topic per section. Put flags, config keys and codes in reference tables near the top or
  bottom, and details that rarely matter in a short "Details" or "Limitations" list at the end.
- Keep headings that other pages link to. Before renaming or removing one, run
  `grep -rn "<page>#" website/src` and update the links.

## Tables

- Use a table for anything that varies by code, flag or policy: meaning, defaults and caveats.
  Don't write a paragraph per code.
- Use tables for flags, config keys with defaults, exit codes, diagnostic codes and fields.
- Keep cells short. Put a long explanation in the text below the table.

## Sentences

- Write short, plain sentences in the present tense. One idea per sentence.
- Give every diagnostic an example where it matters, such as the SQL that triggers it and the SQL
  that fixes it.
- Use neutral orders-style examples (orders, customers, products, payments).

## No history

- Describe the product as it is. Don't write "now", "no longer", "used to", "earlier releases",
  "as before", "upgrade note" or "previously". History belongs in `CHANGELOG.md`.
- Drop notes about legacy syntax and migrations from old versions once the error message itself
  tells the user what to do.

## Changing a page

- When a feature changes a page, edit the page into shape: put the new fact in the section it
  belongs to, update tables and examples, and remove what is no longer true. Don't append a
  paragraph to the end of a section or page.
- Don't repeat a rule that another page owns. Link to it instead, for example to
  `policy#sessions` for session parameters.

## Measured figures

- State only measured figures, each with its basis: the commit, the policy and the command that
  reproduces it. Never present a figure as exhaustive or as proof for every input.
- `diagnostics/reference.mdx` is generated from the code registry by
  `scripts/generate_diagnostics.py`; edit the registry, not the page.
