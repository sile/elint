-module(ok).

-export([case_only/1, simple/0]).


case_only(N) ->
    case N > 0 of
        true ->
            positive;
        false ->
            non_positive
    end.


simple() ->
    ok.
