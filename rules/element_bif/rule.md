# `element_bif`

Disallow `element/2` with a literal index; use pattern matching instead.

## What it does

This rule reports unqualified `element/2` and `erlang:element/2` when the first
argument is an integer literal.

It does not report:

- a non-literal (dynamic) index
- a remote call whose module is not `erlang`
- a call whose arity is not 2

## Why restrict this?

A literal index hides the expected tuple shape and the meaning of each field.
Pattern matching makes that contract visible at the call site.

## Example

```erlang
-module(example).

first(Tuple) ->
    element(1, Tuple).
```

Use instead:

```erlang
-module(example).

first({X, _, _}) ->
    X.
```

## Known limitations

The rule does not decide whether the tuple shape is fixed. A literal index into
a tuple of unknown size is still reported. Suppress the finding if that use is
intentional.

The rule does not resolve callees. An unqualified `element/2` that is no longer
the BIF because of `no_auto_import` and a local definition is still reported.
Suppress the finding if that use is intentional.
