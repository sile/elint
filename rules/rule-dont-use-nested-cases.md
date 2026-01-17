# RULE: TODO

- Context: expr

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

## OK

Use `maybe` instead.

```erlang
maybe
  {tag0, P0_ok} ?= {tag0, V0},
  {tag1, P1_ok} ?= {tag1, V1},
  '$ok_block'
else
  {tag0, P0_error} ->
    '$error_block_0';
  {tag1, P1_error} ->
    '$error_block_1'
end.
```

NOTE:
- Tag names should concise and reflect the context
