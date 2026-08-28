# Diagnostics

How elint reports problems, and what it deliberately does not report.

## Output format

Diagnostics that have a source location use a rustc-style block:

```text
error[element_bif]: Disallow `element/2` with a literal index; use pattern matching instead.
  --> path/to/file.erl:3:5 (in foo/1)
   |
   | foo(T) ->
 3 |     element(1, T)
   |     ^^^^^^^^^^^^
   |
```

- `error[code]`: the bracketed code is the lint rule name. Lint findings
  carry this code; every other diagnostic carries no code
  (`error: message`).
- `--> path:line:col`: 1-based line and character column of the report.
  When a rule finding lies inside a function, the enclosing function name
  (`in foo/1`) is appended to the location line.
- A source line with a caret spanning the reported byte range. When the
  reported range crosses lines, its first and last lines are each shown with
  a caret.
- Colors are enabled only when stderr is a terminal and the `NO_COLOR`
  environment variable is not set.

Errors without a source location, such as a path that does not exist or a
file that cannot be read, are printed as a single plain line and carry no
source block.

All problems are written to stderr. When at least one error is reported,
elint exits with status 1 and prints `Found {n} error(s)`.

## Notes

- The first finding of each lint rule in a file prints a pointer to the
  rule's description: run `elint --explain <name>` for details.
- `-elint_expect` diagnostics (a malformed attribute, or an expectation that
  never matched a finding) print a pointer to
  [elint_expect_attr](explain/elint_expect_attr.md):
  run `elint --explain elint_expect_attr` for details.

Which problems each analysis stage (tokenize, preprocess, parse) reports is
described in [preprocessing](preprocessing.md).

## `-error` / `-warning` directives are ignored

`-error` and `-warning` directives are deliberately not reported; see
[preprocessing](preprocessing.md) for the reason.
