foo(A, B, C) ->
    case A of
        error ->
            error;
        ok ->
            case B of
                error ->
                    error;
                {ok, D} ->
                    {ok, {C, D}}
            end
    end.
