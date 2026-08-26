-module(ng).

-export([function_clause/0,
         case_clause/0,
         if_clause/0,
         receive_after/0,
         try_clauses/0,
         maybe_else/0,
         nested/0,
         anon_fun/0]).


function_clause() -> ok.


case_clause() ->
    case 1 of
        1 -> ok
    end.


if_clause() ->
    if
        true -> ok
    end.


receive_after() ->
    receive
    after
        %% @efmt:off
        0 -> ok
        %% @efmt:on
    end.


try_clauses() ->
    try 1 of
        1 -> ok
    catch
        _:_ -> ok
    end.


maybe_else() ->
    maybe
        ok
    else
        _ -> ok
    end.


nested() ->
    case 1 of
        1 ->
            case 2 of
                2 -> ok
            end
    end.


anon_fun() ->
    fun() -> ok
    end.
