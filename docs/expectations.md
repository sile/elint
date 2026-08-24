# Expectations and suppression

An `-elint_expect` attribute declares that a finding is intentional. elint
suppresses findings that match an expectation, and reports an expectation
that never matched a finding as `Lint Expectation Not Met`.

```erlang
-elint_expect(element_bif, {function, foo, 1}, "dynamic tuple shape").
```

The payload is a tuple `{Rule, {function, Name, Arity}, Reason}`:

- `Rule` is the atom name of the lint rule.
- `{function, Name, Arity}` selects the target: a finding is suppressed when
  it lies inside one of the `Name/Arity` clauses. The target tuple is
  open-ended; other tags such as `{module, M}` or `{record, R}` may be added
  later.
- `Reason` is a required string explaining why the finding is acceptable. A
  declaration without a reason is an error, as are an unknown rule name and
  a function that does not exist in the file.

One attribute covers one rule for one function. Write several attributes to
suppress several rules, or several functions, in the same file.

Expectations are read from the mainline branch only (the branch that takes
`Branch::Then` at every conditional), while findings from every branch are
matched against them. Matching is done on the original-file byte span, so an
expectation also covers findings that a non-mainline conditional arm would
produce inside the target function.
