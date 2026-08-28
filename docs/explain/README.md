# Shared explanations

`docs/explain/` holds shared explanations that lint rules or dedicated
diagnostics point to with `elint --explain <name>`. Each file is embedded in
the binary, and the file stem is the name passed to `--explain`.

## What belongs here

A candidate for `docs/explain/` is a document that:

- is referenced by more than one lint rule, or
- describes an elint-specific attribute or notation, or
- is pointed to directly by a dedicated diagnostic.

Every new explanation must have at least one concrete referrer: a
`rules/<name>/rule.md` section or a diagnostic that links to it. General
usage, diagnostic format, and internal design documents that are fine to read
on GitHub do not belong here, because there is no single diagnostic that
uniquely leads to them.

A description that belongs to exactly one lint rule stays in
`rules/<name>/rule.md`.

## Adding an explanation

1. Name the file `lower_snake_case.md`; the stem is the `--explain` name.
2. Make sure the stem does not collide with a lint rule name or an existing
   shared explanation.
3. Update the explanation registry (`EXPLAINS` in `src/main.rs`), the `--list`
   output, the duplicate check, and the display tests.

## Linking from diagnostics

Ordinary lint findings point only to the description of the rule they
violate. Do not list shared explanations unconditionally in every finding.

Features that require judgment, such as suppression, are pointed to only from
the rule descriptions where using them is appropriate.

`README.md` itself is operational documentation. It is never embedded in the
binary, and it is not a `--explain` target or a `--list` entry.

## Current explanations

- `elint_expect_attr`: the `-elint_expect` attribute and suppression. This is
  the first explanation added under these rules.
