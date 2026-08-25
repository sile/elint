-module(ng).

-export([list/0, bitstring/0, map/1]).

list() ->
    [X || X <- [1, 2, 3]].

bitstring() ->
    [X || <<X:8>> <= <<1, 2, 3>>].

map(Map) ->
    #{K => V || K := V <- Map}.
