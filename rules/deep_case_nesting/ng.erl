-module(ng).

-export([deep/3, deeper/5]).

deep(A, B, C) ->
    case A of
        a1 ->
            case B of
                b1 ->
                    case C of
                        c1 ->
                            ok
                    end
            end
    end.

deeper(A, B, C, D, E) ->
    case A of
        a1 ->
            case B of
                b1 ->
                    case C of
                        c1 ->
                            case D of
                                d1 ->
                                    case E of
                                        e1 ->
                                            ok
                                    end
                            end
                    end
            end
    end.
