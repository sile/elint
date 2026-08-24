# `newline_after_arrow`

Require a newline immediately after `->` in clause bodies.

## What it does

This rule reports a `->` that introduces a clause body when the original
source between that arrow and the first lexical token of the body contains
no newline.

It applies to:

- function clauses
- `case` / `receive` / `fun` / `maybe else` / pattern-only `try catch` clauses
- `if` clauses
- class-qualified `try catch` clauses
- `receive after` sections

It does not report:

- a `->` that already has a newline before the body (including `-> % note`
  followed by a newline, then the body)
- `->` in `-spec` / `-callback` payloads or other attribute payloads
- type `fun((...) -> ...)` arrows
- map `=>` and `maybe` `?=`
- `->` tokens that did not originate in the source file (macro expansion)
- a `->` in an anonymous or named `fun` whose entire `fun ... end` expression
  is on a single line

## Why restrict this?

Putting the body on the same line as `->` leaves a choice between inline and
wrapped form. Wrapping later, or lining up body columns, churns diffs.
Requiring a newline after `->` removes that choice.

## Example

```erlang
-module(example).

first({X, _, _}) -> X.
```

Use instead:

```erlang
-module(example).

first({X, _, _}) ->
    X.
```
