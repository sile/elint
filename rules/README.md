# Rules

Each lint lives under `rules/<name>/` and `src/rules/<name>.rs`.
`<name>` is a `lower_snake_case` atom. Do not use hyphens.

Lint rules assume Erlang/OTP 29.0 or later. A rule may rely on syntax
introduced in OTP 29.0 without documenting a version restriction.

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

An example that is reported (self-contained for `elint --explain <name>`).

Use instead:

An example of the preferred alternative.

## Known limitations

Acceptable uses that are still reported and therefore require suppression.
Omit this section when the rule has none.

Point to a shared explanation (for example `elint --explain elint_expect_attr`)
only when the rule has a legitimate intentional exception or known limitation.
Do not add suppression boilerplate shared by every rule. When the rule does
point to one, first explain in which cases suppression is appropriate, then
give the shared explanation or a concrete `-elint_expect` attribute.
```

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

`elint --explain <name>` prints `rules/<name>/rule.md`.

Shared explanations (such as the `-elint_expect` notation) live under
`docs/explain/` and are printed with `elint --explain <name>`; run
`elint --list` for the list. Link to a shared explanation from a rule's
`Known limitations` only when that rule has legitimate intentional uses; see
`docs/explain/README.md` for the placement rules.
