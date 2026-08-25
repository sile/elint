# `deep_case_nesting`

Flag case nested three or more levels deep; consider maybe or splitting the function.

## What it does

This rule reports a `case` that is nested three or more levels deep. The depth
counts consecutive `case` expressions; an intervening `fun`, `try`, `receive`,
`begin`, `maybe`, container literal (list / tuple / map / bitstring), or
comprehension resets the depth to zero.

It does not report:

- a `case` nested fewer than three levels deep
- a `case` whose run of `case` expressions was broken by one of the
  expressions above

## Why restrict this?

Deeply nested `case` expressions are hard to follow: each level adds another
axis along which the flow can diverge, and the outer branches sit far from the
inner logic they control. `maybe` or splitting the function into smaller ones
flattens the flow and makes the branches easier to reason about.

## Example

```erlang
-module(example).

handle(A, B, C) ->
    case A of
        a ->
            case B of
                b ->
                    case C of
                        c ->
                            ok;
                        _ ->
                            error
                    end;
                _ ->
                    error
            end;
        _ ->
            error
    end.
```

Use instead:

```erlang
-module(example).

handle(A, B, C) ->
    maybe
        a ?= A,
        b ?= B,
        c ?= C,
        ok
    else
        _ ->
            error
    end.
```
