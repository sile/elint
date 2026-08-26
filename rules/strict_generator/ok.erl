-module(ok).

-export([list/0, bitstring/0, map/1, zip/0]).


list() ->
    [ X || X <:- [1, 2, 3] ].


bitstring() ->
    [ X || <<X:8>> <:= <<1, 2, 3>> ].


map(Map) ->
    #{ K => V || K := V <:- Map }.


zip() ->
    [ {X, Y} || X <:- [1, 2] && Y <:- [3, 4] ].
