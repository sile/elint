# RULE: Don't Use BIF `element()` for Tuple Access, Use Pattern Matching Instead

## NG

Direct BIF call to access a tuple element.

```erlang
foo(Tuple) ->
  element(1, Tuple).
```

or

```erlang
foo(Tuple) ->
  erlang:element(1, Tuple).
```

## OK

Use pattern matching instead.

```erlang
foo({X, _Y, _Z}) ->
  X.
```

or

```erlang
foo(Tuple) ->
  {X, _Y, _Z} = Tuple,
  X.
```

## EXCEPTION

If the exact size of the tuple is unknown or dynamically varies, using `element()` is acceptable.
