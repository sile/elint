# RULE: TODO

- Context: expr

## NG

Nested `case`s. 

```erlang
case '$0' of
  {ok, '$ok_0'} ->
    case '$1' of
      {ok, '$ok_1'} ->
          '$ok_block';
      '$error_1' -> 
          '$error_block1'
    end;
  '$error_0' -> 
      '$error_block_0'
end.
```

## OK

Use `maybe` instead.

```erlang
maybe
  {tag0, {ok, '$ok_0'}} ?= {tag0, '$0'},
  {tag1, {ok, '$ok_1'}} ?= {tag1, '$1'},
  '$ok_block'
else
  {tag0, '$error_0'} ->
    '$error_block_0';
  {tag1, '$error_1'} ->
    '$error_block_1'
end.
```
