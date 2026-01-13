# LINT RULE: foo

## NG

```erlang
case '$0' of
  error -> '$1';
  {error, '$2'} -> '$3':
end.
```

## OK


```erlang

```
