-module(ok).

-export([f/0]).


f() ->
    ok.


-ifdef(TEST).
%% Test-only API.
-export([unix_time/1]).


unix_time(_) ->
    0.


-endif.
