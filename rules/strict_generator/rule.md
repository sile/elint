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

## Caution: a relaxed generator may be a filter

A relaxed generator silently drops elements that do not match its pattern, so
in existing code it is often used as a filter as well as a generator. Replacing
such a generator with a strict one is not equivalent: the first non-matching
element now raises `badmatch` instead of being skipped, so the rewrite changes
behavior and can crash the program. Only replace a relaxed generator with a
strict one when every element of the source is expected to match the pattern.

When the relaxed generator filters, make the filtering explicit. If the
matching shapes share a pattern, bind the discriminating field and add a
filter:

```erlang
% before
[V*V || #foo{flag=true, value=V} <- List]
% after
[V*V || #foo{flag=Flag, value=V} <:- List, Flag =:= true]
```

The filter must compare with `=:= true`; a bare `Flag` would also pass values
such as `undefined` that the original pattern match rejected.

When the source mixes shapes with no common pattern, no direct rewrite exists.
For example, if `List` is `[{ok, term()} | error]`, the strict generator
`{ok, V} <:- List` raises `badmatch` on every `error` element. When performance
is not critical, filter first and then use a strict generator:

```erlang
[V*V || {ok, V} <:- lists:filter(fun ({ok, _}) -> true; (error) -> false end, List)]
```

This materializes an intermediate list that the relaxed generator avoided, so
it is a performance trade-off.

## Known limitations

A relaxed generator used intentionally as a filter is still reported. When the
relaxed form is noticeably simpler or faster than any strict equivalent,
keeping it is reasonable. Declare the intent with an `-elint_expect` attribute
(see `elint --explain elint_expect_attr`), and add a comment next to the
generator if the reason is not obvious:

```erlang
-elint_expect(strict_generator, {function, foo, 1}, "filters ok/error pairs; strict would raise badmatch").
```
