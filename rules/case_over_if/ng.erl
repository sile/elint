-module(ng).

-export([basic/1, with_true/0, nested/1]).


basic(N) ->
    if
        N > 0 ->
            positive;
        true ->
            non_positive
    end.


with_true() ->
    if
        true ->
            ok
    end.


nested(N) ->
    case N of
        0 ->
            if
                true ->
                    zero
            end;
        _ ->
            other
    end.
