# `ifdef_test_at_end`

Require one `-ifdef(TEST)` block at the end of the file, with no `-else` or `-elif`.

## What it does

This rule reports an `-ifdef(TEST).` directive unless it is the only
`-ifdef(TEST)` in the file and its matching `-endif.` is the last lexical
token. Whitespace and comments may follow that `-endif.`.

It also reports that directive when `-else` or `-elif` belongs to the
`-ifdef(TEST)` itself, including when the `-endif.` is already at the end of
the file. `-else` and `-elif` that belong to a nested conditional do not
count. An `-ifdef(TEST)` with no matching `-endif` is reported.

It does not report:

- a file with no `-ifdef(TEST)`
- `-ifndef(TEST)`, `-if(...)`, and `-ifdef` of any other macro
- the contents of the block

`attr_order` still applies inside the block. It allows `-export` and
`-export_type` there, and still reports `-type`, `-opaque`, and `-record`
after the first function.

## Why restrict this?

Test-only exports, includes, and functions are easiest to find when they
share one block at the end of the module. An `-ifdef(TEST)` in the header or
between functions splits that test surface across the file. An `-else` arm on
`-ifdef(TEST)` mixes production code into the same conditional, so the block
is no longer a test-only tail.

## Example

```erlang
-module(example).

-export([f/0]).

-ifdef(TEST).
-export([unix_time/1]).
-endif.

f() ->
    ok.
```

Use instead:

```erlang
-module(example).

-export([f/0]).

f() ->
    ok.

-ifdef(TEST).
-export([unix_time/1]).
-endif.
```

## Known limitations

An `-else` that selects a production implementation, and a test-only section
in the middle of a header, are still reported. When the split is intentional,
suppress it with an `-elint_expect` attribute (see
`elint --explain elint_expect_attr`):

```erlang
-elint_expect(ifdef_test_at_end, module, "production timeout differs under TEST").
```

A nested `-ifdef(TEST)` inside an otherwise trailing block is also reported,
because the inner directive is not itself the file suffix.
