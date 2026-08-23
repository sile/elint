-module(ok).

-export([other_module/1, dynamic_index/2, wrong_arity/1, pattern/1]).

other_module(Tuple) ->
    lists:element(1, Tuple).

dynamic_index(N, Tuple) ->
    element(N, Tuple).

wrong_arity(Tuple) ->
    element(1, Tuple, extra).

pattern({X, _, _}) ->
    X.
