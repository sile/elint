-module(ng).

-export([f/0]).

-ifdef(TEST).
-export([unix_time/1]).
-endif.


f() ->
    ok.
