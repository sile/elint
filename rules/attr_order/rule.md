# `attr_order`

Keep module attributes in conventional header order; do not place exports, types, or records after functions.

## What it does

This rule reports two kinds of violations:

1. **Header order.** Before the first function clause, the first occurrence of
   each attribute class must follow this order:
   - `-module`
   - `-behaviour` / `-behavior`
   - `-export` / `-export_type`
   - `-include` / `-include_lib`
   - `-record` / `-type` / `-opaque` / `-define` (any order within this group)

   A later class that appears before an earlier class is reported (for example
   `-export` after `-include`, or `-include` after `-type`).

2. **Placement after functions.** After the first function clause:
   - `-type` / `-opaque` / `-record` / `-export` / `-export_type` are always
     reported
   - `-define` / `-include` / `-include_lib` are reported unless they lie inside
     an `-ifdef(TEST).` ... `-endif.` then-arm (up to a possible `-else`)

It does not report:

- `-spec` / `-callback` next to functions
- Unclassified attributes such as `-compile`, `-doc`, or `-elint_expect`
- Relative order among `-record`, `-type`, `-opaque`, and `-define`
- `-define` / `-include` / `-include_lib` inside `-ifdef(TEST)` after functions

`-include` / `-include_lib` are skipped by preprocessing and do not remain in
the parse tree. This rule recovers those directive sites from the preprocessor
so their order and placement can still be checked.

## Why restrict this?

Module headers are easiest to read when exports, includes, and type-level
declarations stay at the top in a stable order. Attributes that sit between
function clauses split the module's surface and make the header an incomplete
summary. Test-only macros and eunit includes conventionally live under
`-ifdef(TEST)`, so that placement remains allowed.

## Example

```erlang
-module(example).

-include("example.hrl").

-export([f/0]).

f() ->
    ok.

-type t() :: ok.
```

Use instead:

```erlang
-module(example).

-export([f/0]).

-include("example.hrl").

-type t() :: ok.

f() ->
    ok.

-ifdef(TEST).
-include_lib("eunit/include/eunit.hrl").
-endif.
```

## Known limitations

Only `-ifdef(TEST)` then-arms are recognized as test regions. `-ifndef(TEST)`,
`-if(...)`, and other macros are not treated as exemptions.
