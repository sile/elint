-module(ok).

-behaviour(gen_server).

-export([f/1]).

-export_type([t/0]).

-include("ok.hrl").

-include_lib("kernel/include/logger.hrl").

-record(state, {
          value :: t()
         }).

-type t() :: atom().

-define(DEFAULT, none).


-spec f(t()) -> t().
f(Value) ->
    Value.


-ifdef(TEST).
-define(TEST_ONLY, 1).
-include_lib("eunit/include/eunit.hrl").
-endif.
