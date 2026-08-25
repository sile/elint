# `case_over_if`

Prefer case over if.

## What it does

This rule reports every `if` expression.

It does not report `case` expressions or any other construct.

## Why restrict this?

An `if` branch can only contain a guard, so it cannot match a pattern or bind
a value. Every `if` can be rewritten as a `case` (typically `case true of
true when Guard -> ... end`), which is more expressive and the conventional
choice.

## Example

```erlang
-module(example).

classify(N) ->
    if
        N < 0 ->
            negative;
        N > 0 ->
            positive;
        true ->
            zero
    end.
```

Use instead:

```erlang
-module(example).

classify(N) ->
    case N of
        0 ->
            zero;
        _ when N > 0 ->
            positive;
        _ ->
            negative
    end.
```
