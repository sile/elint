# Diagnostics

How elint reports problems, and what it deliberately does not report.

## Output format

Every report shares one rustc-style block:

```text
error[element_bif]: Disallow `element/2` with a literal index; use pattern matching instead.
  --> path/to/file.erl:3:5 (in foo/1)
   |
   | foo(T) ->
 3 |     element(1, T)
   |     ^^^^^^^^^^^^
   |
```

- `error[code]`: the bracketed code is the lint rule name. Other reports
  carry no code (`error: message`).
- `--> path:line:col`: 1-based line and character column of the report.
  When a rule finding lies inside a function, the enclosing function name
  (`in foo/1`) is appended to the location line.
- A source line with a caret spanning the reported byte range (for a
  multi-line range, only its first line is shown). Tabs are expanded to
  four columns so the caret stays aligned.
- When the reported line is not the first line, the immediately preceding
  line is shown without a line number as context; an empty preceding line
  is omitted.
- Colors are enabled only when stderr is a terminal and the `NO_COLOR`
  environment variable is not set.

Any report makes elint exit with status 1 and print `Found {n} error(s)`.

## What each stage reports

- **tokenize**: a lexical error aborts the file; no branch is linted.
- **preprocess**: structural errors of the preprocessor itself (stray
  `-else`, unclosed conditional, macro arity mismatch, ...) are reported
  per branch. The branch's lint still runs if the file parses.
- **parse**: syntax errors are reported per branch. That branch's lint is
  skipped; other branches still contribute findings.
- **lint**: rule findings, unless suppressed by an `-elint_expect`
  attribute (see [expectations](expectations.md)). The first occurrence of
  a rule also prints a suppression hint and a pointer to `elint explain`.
- **expectations**: malformed `-elint_expect` attributes and expectations
  that never matched a finding are reported (see
  [expectations](expectations.md)).

## `-error` / `-warning` directives are ignored

elint scans every arm of every conditional (see
[preprocessing](preprocessing.md)). A compiler takes one branch per
conditional and only meets the directives in the branches it selects; elint
would meet `-error` / `-warning` in every arm it scans, so reporting them
would be noisy. Judging these directives is the compiler's job, not a
linter's, so elint ignores them entirely.
