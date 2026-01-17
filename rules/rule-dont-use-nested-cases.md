# RULE: TODO

- CONTEXT: EXPR

## NG

Nested `case`s. 

```erlang
case Value0 of
  OkPattern0 ->
    case Value1 of
      OkPattern1 ->
          OkResult;
      ErrorPattern1 -> 
          ErrorResult1
    end;
  ErrorPattern0 -> 
      ErrorResult0
end.
```

### WHEN: `OkPattern0`, `OkPattern0`

- MATCH: `ok`
- IS_TUPLE:
  - TAG: `ok`

## OK

Use `maybe` instead.

```erlang
maybe
  {tag0, OkPattern0} ?= {tag0, Value0},
  {tag1, OkPattern1} ?= {tag1, Value1},
  '$ok_block'
else
  {tag0, ErrorPattern0} ->
    '$error_block_0';
  {tag1, ErrorPattern1} ->
    '$error_block_1'
end.
```

NOTE:
- Tag names should concise and reflect the context
