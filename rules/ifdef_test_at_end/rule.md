# `ifdef_test_at_end`

Place `-ifdef(TEST)` only once, at the end of the file.

## What it does

This rule reports an `-ifdef(TEST).` directive unless it is the only
`-ifdef(TEST)` in the file and its matching `-endif.` is the last lexical
token. Whitespace and comments may follow that `-endif.`.

An `-ifdef(TEST)` with no matching `-endif` is also reported.

It does not report:

- a file with no `-ifdef(TEST)`
- `-ifndef(TEST)`, `-if(...)`, and `-ifdef` of any other macro
- the contents of the block

## Why restrict this?

Test-only definitions are easiest to find when they share one block at the
end of the module. An `-ifdef(TEST)` in the header or between functions
splits that test surface across the file.

A function exported only for tests is a special case. Export it
unconditionally in the module header and add a comment that explains why it
is exported. Do not wrap the export in `-ifdef(TEST)`.

## Example

```erlang
-module(example).

-ifdef(TEST).
-export([f/0]).
-endif.

f() ->
    ok.
```

Use instead:

```erlang
-module(example).

%% Exported for tests.
-export([f/0]).

f() ->
    ok.
```
