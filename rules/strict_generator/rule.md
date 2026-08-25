# `strict_generator`

Require strict comprehension generators (<:- / <:=) instead of <- / <=.

## What it does

This rule reports the relaxed list generator `<-`, the relaxed bit string
generator `<=`, and the relaxed map generator `<-` inside a comprehension, and
suggests the strict variants `<:-` and `<:=`.

It does not report:

- generators that already use the strict arrows `<:-` / `<:=`
- generators inside a zip generator (`&&`)

## Why restrict this?

A relaxed generator silently drops elements that do not match its pattern,
hiding unexpected input. A strict generator raises `badmatch` on the same
mismatch, so incorrect input becomes visible instead of quietly disappearing.
The intent is also clearer with a strict generator: a relaxed generator can be
either a generator or an accidental filter, and the code alone does not tell
which it is (EEP-0070).

## Example

```erlang
-module(example).

names() ->
    [X || {X, _} <- pairs()].
```

Use instead:

```erlang
-module(example).

names() ->
    [X || {X, _} <:- pairs()].
```

## Known limitations

A relaxed generator used intentionally as a filter is still reported. Suppress
the finding if that use is intentional.
