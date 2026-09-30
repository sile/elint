-module(ok).

-export([f/0]).

%% Exported for tests.
-export([unix_time/1]).


f() ->
    ok.


unix_time(_) ->
    0.


-ifdef(TEST).


f_test() ->
    ok.


-endif.
