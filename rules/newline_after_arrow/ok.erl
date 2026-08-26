-module(ok).

-export([function_clause/0,
         case_clause/0,
         if_clause/0,
         receive_after/0,
         try_clauses/0,
         maybe_else/0,
         anon_fun/0,
         map_and_maybe_match/0,
         comment_then_newline/0,
         one_line_anon_fun/0,
         one_line_named_fun/0]).


-spec function_clause() -> ok.

function_clause() ->
    ok.


case_clause() ->
    case 1 of
        1 ->
            ok
    end.


if_clause() ->
    if
        true ->
            ok
    end.


receive_after() ->
    receive
    after
        0 ->
            ok
    end.


try_clauses() ->
    try 1 of
        1 ->
            ok
    catch
        _:_ ->
            ok
    end.


maybe_else() ->
    maybe
        ok
    else
        _ ->
            ok
    end.


anon_fun() ->
    fun() ->
            ok
    end.


one_line_anon_fun() ->
    fun() -> ok end.


one_line_named_fun() ->
    fun F() -> ok end.


map_and_maybe_match() ->
    _ = #{a => b},
    maybe
        X ?= {ok, 1},
        X
    else
        _ ->
            error
    end.


comment_then_newline() ->  % note
    ok.


-define(MACRO_CLAUSE, macro_clause() -> ok).


?MACRO_CLAUSE.
