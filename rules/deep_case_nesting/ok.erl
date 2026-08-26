-module(ok).

-export([two_levels/1, fun_boundary/1, block_boundary/1, single/1]).


two_levels(X) ->
    case X of
        1 ->
            case X of
                2 ->
                    ok
            end
    end.


fun_boundary(X) ->
    case X of
        1 ->
            fun() ->
                    case X of
                        2 ->
                            case X of
                                3 ->
                                    ok
                            end
                    end
            end()
    end.


block_boundary(X) ->
    case X of
        1 ->
            try
                case X of
                    2 ->
                        case X of
                            3 ->
                                ok
                        end
                end
            catch
                _:_ ->
                    error
            end
    end.


single(X) ->
    case X of
        1 ->
            ok
    end.
