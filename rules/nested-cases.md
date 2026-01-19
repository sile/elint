# RULE: Don't Use Nested Cases, Use `maybe` Instead

## NG

Nested `case`s. 

```erlang
case Result0 of
  Ok0 ->  % Ok0 = ok | {ok, ...}
    case Result1 of
      Ok1 ->  % Ok1 = ok | {ok, ...}
          '...0';
      Error1 ->  % Error1 = error | {error, ...}
          '...2'
    end;
  Error0 ->  % Error0 = error | {error, ...}
      '...1'
end.
```

## OK

Use `maybe` instead.

```erlang
maybe
  {tag0, Ok0} ?= {tag0, Value0},
  {tag1, Ok1} ?= {tag1, Value1},
  '...0'
else
  {tag0, Error0} ->
    '...1';
  {tag1, Error1} ->
    '...2'
end.
```

NOTE:
- Tag names should be concise and reflect the context
