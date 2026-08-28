# elint

[![crates.io](https://img.shields.io/crates/v/elint.svg)](https://crates.io/crates/elint)
[![CI](https://github.com/sile/elint/workflows/CI/badge.svg)](https://github.com/sile/elint/actions)
[![License](https://img.shields.io/crates/l/elint)](LICENSE)
[![Documentation](https://docs.rs/elint/badge.svg)](https://docs.rs/elint)

An Erlang code linter.

Lint rules assume Erlang/OTP 29.0 or later.

## Features

- Lints both `.erl` and `.hrl` files.
- Accepts multiple files or directories and walks directories recursively.
- Analyzes every arm of every conditional, including arms the current macro
  state would not select.
- Embeds the description of every lint rule in the binary and prints it with
  `--explain`.
- Lets you declare intentional findings with `-elint_expect`, and reports
  expectations that never matched a finding.

Run `elint --list` for the available lint rules and shared explanations;
`elint --explain <name>` prints one.

## Example

```erlang
-module(example).

first(Tuple) ->
    element(1, Tuple).
```

```console
$ elint --lint element_bif example.erl
error[element_bif]: Disallow `element/2` with a literal index; use pattern matching instead.
 --> example.erl:4:5 (in first/1)
  |
  | first(Tuple) ->
4 |     element(1, Tuple).
  |     ^^^^^^^^^^^^^^^^^
  |
note: run `elint --explain element_bif` for details
Found 1 error(s)
```

The report shows the rule name in brackets, the location
(`path:line:col`, plus the enclosing function), the offending source line,
and a caret over the reported span. The note points to
`elint --explain element_bif` for the rule description. elint exits with
status 1 because at least one problem was found.

## Installation

### Pre-built binaries

Pre-built binaries for Linux and macOS are available from the
[releases page](https://github.com/sile/elint/releases):

- `x86_64-unknown-linux-musl` (Linux x86_64, fully static)
- `aarch64-unknown-linux-musl` (Linux arm64, fully static)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)

For example, download the Linux x86_64 binary with the GitHub CLI:

```console
$ VERSION=0.1.0
$ gh release download v${VERSION} --repo sile/elint \
    --pattern "elint-${VERSION}.x86_64-unknown-linux-musl"
$ chmod +x elint-${VERSION}.x86_64-unknown-linux-musl
$ ./elint-${VERSION}.x86_64-unknown-linux-musl --version
```

Or with `curl`:

```console
$ VERSION=0.1.0
$ curl -L https://github.com/sile/elint/releases/download/v${VERSION}/elint-${VERSION}.x86_64-unknown-linux-musl -o elint
$ chmod +x elint
$ ./elint --version
```

The asset name is `elint-<version>.<target>`, where `<target>` is one of the
triples above and `<version>` is the release version without the leading `v`.

### With Cargo

If you have `cargo` (the Rust package manager) installed:

```console
$ cargo install elint
$ elint --version
```

See the [Rust documentation](https://doc.rust-lang.org/cargo/) to install
Cargo itself.

## Usage

```console
$ elint [OPTIONS] [PATH]..
```

With no `PATH`, elint lints `src/` and `tests/`. You can pass multiple files
or directories; directories are walked recursively, and `.erl` / `.hrl`
files are linted.

```console
$ elint
$ elint src
$ elint src tests
$ elint path/to/module.erl
```

A path that you specify explicitly must exist; elint reports it as an error
otherwise.

For the other command-line options, see the help:

```console
$ elint -h
$ elint --help
```

## Selecting lint rules

`-l` / `--lint <RULE>` restricts linting to the named rule and may be
repeated:

```console
$ elint --lint element_bif src
$ elint --lint element_bif --lint strict_generator src
```

Run `elint --list` for the available rule names.

## Explanations

`--list` lists the available lint rules and shared explanations, and
`--explain <name>` prints one:

```console
$ elint --list
$ elint --explain element_bif
$ elint --explain elint_expect_attr
```

A normal lint finding points to the description of the rule it violates.
`elint_expect_attr` is a shared explanation referenced from rules with
legitimate intentional exceptions and from `-elint_expect` diagnostics.
`--explain` covers rule descriptions and the shared explanations under
`docs/explain/`; it is not a viewer for every file in `docs/`.

## Documentation

- [Preprocessing](docs/preprocessing.md): how elint preprocesses Erlang
  source, including conditional-branch exploration and include resolution
  policy.
- [Diagnostics](docs/diagnostics.md): how elint reports problems and why
  `-error` / `-warning` directives are ignored.
- [The `-elint_expect` attribute](docs/explain/elint_expect_attr.md): the
  `-elint_expect` notation and suppression.

`docs/diagnostics.md` and `docs/preprocessing.md` are meant to be read on
GitHub; they are not `--explain` targets.

## Suppressing intentional findings

When a finding is intentional, declare it with an `-elint_expect` attribute
so elint suppresses it and reports an expectation that never matched:

```console
$ elint --explain elint_expect_attr
```

See [the `-elint_expect` attribute](docs/explain/elint_expect_attr.md) for
the notation.

## Diagnostics and exit status

Findings and analysis errors are written to stderr. elint exits with status
1 when at least one problem is reported, and status 0 when there are none.
See [docs/diagnostics.md](docs/diagnostics.md) for the output format.
