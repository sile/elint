# RULE: Don't Use Nested Cases, Use `maybe` Instead

- CONTEXT: EXPR

## NG

Nested `case`s. 

```erlang
case Value0 of
  OkPattern0 ->  % IF_MATCH: OkPattern0 = ok | {ok, '...'}
    case Value1 of
      OkPattern1 ->  % IF_MATCH: OkPattern1 = ok | {ok, '...'}
          '...0';
      ErrorPattern1 -> 
          '...1'
    end;
  ErrorPattern0 -> 
      '...2'
end.
```

## OK

Use `maybe` instead.

```erlang
maybe
  {tag0, OkPattern0} ?= {tag0, Value0},
  {tag1, OkPattern1} ?= {tag1, Value1},
  '...0'
else
  {tag0, ErrorPattern0} ->
    '...1';
  {tag1, ErrorPattern1} ->
    '...2'
end.
```

NOTE:
- Tag names should be concise and reflect the context
