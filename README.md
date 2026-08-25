# elint

Erlang code linter.

**WIP**

Lint rules assume Erlang/OTP 29.0 or later.

## Documentation

- [Preprocessing](docs/preprocessing.md): how elint preprocesses Erlang
  source, including conditional-branch exploration and include resolution
  policy.
- [Expectations](docs/expectations.md): the `-elint_expect` notation and
  suppression.
- [Diagnostics](docs/diagnostics.md): how elint reports problems and why
  `-error` / `-warning` directives are ignored.

`elint doc` prints any of these documents from the command line.
