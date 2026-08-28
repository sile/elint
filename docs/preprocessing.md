# Preprocessing

How elint preprocesses Erlang source, in terms of what a user can observe.
The implementation is documented in `src/context.rs`.

elint lints `.erl` / `.hrl` files; `collect_erlang_files` in `src/fs.rs`
collects them under a given file or directory.

## Every conditional arm is linted

elint analyzes **every arm of every conditional**, not only the branch the
current macro state would select:

- At each `-ifdef` / `-ifndef` / `-if` / `-elif`, every arm is scanned.
- The **mainline** always takes `Branch::Then` at every conditional and
  continues past each `-endif`. This is a deliberate choice: it is not the
  OTP `recommended` branch, and `-if` / `-elif` conditions are never
  evaluated.
- A non-mainline arm is scanned only up to the `-endif` that closes the
  conditional it was forked from; it does not continue past it.

The arms are not combined as a full product: each source region is scanned
by exactly one branch, so findings do not duplicate across branches.

A parse error in one branch only skips that branch's lint; findings from the
other branches are kept.

## Includes are not resolved

`-include` / `-include_lib` are **not** followed to the filesystem. elint
keeps no search-path configuration (`-I`, `ERL_LIBS`, ...), so headers are
skipped entirely. Macros, records, and types defined in headers are never
seen and are not incorporated into the analysis.

## Unknown macros expand to `elint_dummy`

Unknown macros -- header macros, but also OTP predefined macros such as
`?MODULE` / `?FUNCTION_NAME`, of which `erl_pp` only implements `?FILE` and
`?LINE` -- expand to the placeholder atom `elint_dummy`. The placeholder is
a lowercase atom, so it is valid wherever a macro value could appear and the
file still parses.

No unknown-macro diagnostic is emitted. Without include resolution there is
no way to tell a macro that is genuinely undefined from one defined in an
unresolvable header, so reporting them would be noisy.

The consequence for lint rules is that they cannot see through
header-defined macros or records today. A rule that needs header information
requires include resolution to be reconsidered.

## `-error` / `-warning` are ignored

elint scans every arm of every conditional. A compiler takes one branch per
conditional and only meets the directives in the branches it selects; elint
would meet `-error` / `-warning` in every arm it scans, so reporting them
would be noisy. Judging these directives is the compiler's job, not a
linter's, so elint ignores them entirely.
