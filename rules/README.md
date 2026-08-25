# Rules

Each lint lives under `rules/<name>/` and `src/rules/<name>.rs`.
`<name>` is a `lower_snake_case` atom. Do not use hyphens.

## Layout

| Path | Role |
|---|---|
| `rules/<name>/rule.md` | Description of the rule |
| `rules/<name>/ng.erl` | Fixture that must produce one or more findings for `<name>` |
| `rules/<name>/ok.erl` | Fixture that must produce zero findings for `<name>` |
| `src/rules/<name>.rs` | Implementation |
| `src/rules.rs` | `RULES` registration |

`ng` / `ok` name the expected finding counts (at least one / zero), not whether
the Erlang code is good or bad. `-module` must match the file stem (`ng` /
`ok`). Fixtures are linted one file at a time, so duplicate module names across
rules are fine.

## `rule.md` template

```markdown
# `<name>`

One-sentence summary.

## What it does

The exact conditions under which the rule reports a finding, including nearby
conditions that are not reported.

## Why restrict this?

Why the construct is generally discouraged, without claiming that every use is
incorrect.

## Example

An example that is reported (self-contained for `elint explain`).

Use instead:

An example of the preferred alternative.

## Known limitations

Acceptable uses that are still reported and therefore require suppression.
```

Omit `Known limitations` when the rule has none.

## Adding a rule

1. Create `rules/<name>/rule.md` from the template.
2. Implement `src/rules/<name>.rs` and register it in `RULES`.
3. Add `ng.erl` and `ok.erl`.
4. Check with the linter:

```text
elint --lint <name> rules/<name>/ng.erl
elint --lint <name> rules/<name>/ok.erl
```

`ng.erl` must produce findings and a non-zero exit.
`ok.erl` must produce no findings and a zero exit.

`elint explain <name>` prints `rules/<name>/rule.md`.

Project-wide documents (the `-elint_expect` notation, diagnostics, the
preprocessing pipeline) are available with `elint doc <name>`; run `elint doc`
for the list.
