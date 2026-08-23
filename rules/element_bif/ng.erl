-module(ng).

-export([local/1, remote/1]).

local(Tuple) ->
    element(1, Tuple).

remote(Tuple) ->
    erlang:element(1, Tuple).
