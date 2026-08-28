# elint

Erlang code linter.

**WIP**

Lint rules assume Erlang/OTP 29.0 or later.

## Documentation

- [Preprocessing](docs/preprocessing.md): how elint preprocesses Erlang
  source, including conditional-branch exploration and include resolution
  policy.
- [Diagnostics](docs/diagnostics.md): how elint reports problems and why
  `-error` / `-warning` directives are ignored.
- [The `-elint_expect` attribute](docs/explain/elint_expect_attr.md): the
  `-elint_expect` notation and suppression.

Run `elint --list` to list the available lint rule and shared explanations;
`elint --explain <name>` prints one.
