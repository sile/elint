# Preprocessing

How elint turns an Erlang source file into a parse forest, and two behaviors
of that pipeline that may surprise users coming from a compiler:

- every conditional branch is linted, not only the branch a compiler would
  take;
- `-include` / `-include_lib` are not resolved to the filesystem.

## Pipeline

For each `.erl` file elint runs a three-stage pipeline (`src/context.rs`):

1. **Tokenize** with `erl_tokenize`.
2. **Preprocess** with `erl_pp`. The driver responds to every
   `Awaiting*` event (include, conditional, macro expansion).
3. **Parse** the preprocessed lexical tokens with `erl_parse`
   (`ParseMode::Module`).

The driver may explore several preprocessor states for one file; each
explored state becomes a `BranchContext` (tokens, origin side table, parse
tree, preprocessor diagnostics). The first branch is the mainline. Rules run
independently on each branch, and findings are mapped back to byte spans in
the original file via the origin side table.

## Conditional branch exploration

A compiler picks one branch per conditional according to the current macro
table. A linter instead wants to catch problems in code that the current
configuration would skip. elint therefore analyzes **every arm of every
conditional**:

- At each `-ifdef` / `-ifndef` / `-if` / `-elif`, the preprocessor is cloned
  and each arm is scanned with `Branch::Then` or `Branch::Else`.
- The **mainline** always takes `Branch::Then` at every conditional and
  continues past each `-endif`. This is a deliberate choice: it is not the
  OTP `recommended` branch, and `-if` / `-elif` conditions are never
  evaluated.
- A **non-mainline** arm is scanned only up to the `-endif` that closes the
  conditional it was forked from; the scan stops there.

This stop-at-`-endif` rule is what keeps the cost linear. Each source region
is scanned by exactly one branch, so findings do not duplicate across
branches and the total work is proportional to the file size rather than to
the 2^n product of independent conditionals. Nested conditionals inside an
arm are explored the same way by the branch that made that arm active.

All findings from all branches are reported, mapped to the original file
span of the offending tokens.

## Include resolution

`-include` / `-include_lib` are **not** followed to the filesystem. elint
keeps no search-path configuration (`-I`, `ERL_LIBS`, ...), so headers are
skipped entirely.

The consequences:

- Macros defined in headers are never seen, so they are treated like any
  other unknown macro.
- Unknown macros -- header macros, but also OTP predefined macros such as
  `?MODULE` / `?FUNCTION_NAME`, of which `erl_pp` only implements
  `?FILE` and `?LINE` -- expand to the placeholder atom `elint_dummy`.
  The placeholder is a lowercase atom so it is valid wherever a macro value
  could appear and the file still parses.
- No unknown-macro diagnostic is emitted. Without include resolution there is
  no way to tell a macro that is genuinely undefined from one defined in an
  unresolvable header, so reporting them would be noisy.

The consequence for rules is that they cannot see through header-defined
macros or records today. If a rule appears that needs header information,
include resolution should be reconsidered at that point.

## Reporting

- Tokenize errors abort the file: no branch is linted.
- Preprocessor diagnostics (`-error` / `-warning`, input errors) and parse
  diagnostics are reported per branch.
- Parse diagnostics cause only that branch's lint to be skipped; other
  branches still contribute findings.
