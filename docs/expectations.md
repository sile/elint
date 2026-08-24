# Expectations and suppression

An `-elint_expect` attribute declares that a finding is intentional. elint
suppresses findings that match an expectation, and reports an expectation
that never matched a finding as `Lint Expectation Not Met`.

```erlang
-elint_expect(element_bif, {function, foo, 1}, "dynamic tuple shape").
```

The payload is a tuple `{Rule, Target, Reason}`:

- `Rule` is the atom name of the lint rule.
- `Target` selects the scope. It is either `{function, Name, Arity}`, which
  suppresses findings inside the `Name/Arity` clauses, or the bare atom
  `module`, which suppresses findings anywhere in the current module. The
  target forms are open-ended; other tags such as `{record, R}` may be added
  later.
- `Reason` is a required string explaining why the finding is acceptable. A
  declaration without a reason is an error, as are an unknown rule name and
  a function that does not exist in the file.

One attribute covers one rule for one function. Write several attributes to
suppress several rules, or several functions, in the same file.

When `elint -l RULE` restricts linting to a rule, only expectations for that
rule are validated and reported; `-elint_expect` declarations for other rules
are ignored entirely. An expectation whose rule name cannot be read is an
error regardless of `-l`.

Expectations are read from the mainline branch only (the branch that takes
`Branch::Then` at every conditional), while findings from every branch are
matched against them. Matching is done on the original-file byte span, so an
expectation also covers findings that a non-mainline conditional arm would
produce inside the target function.
