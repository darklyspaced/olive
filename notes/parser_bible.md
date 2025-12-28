5 + 27 + foo() - 7 * alpha()

(((5 + 27) + foo()) - (7 * alpha()))

BIN_EXP
    BIN_EXP
        BIN_EXP
            5
            +
            27
        +
        FN_APP
            FOO
            PARAM_LIST
    -
    BIN_EXP
        7
        *
        FN_APP
            ALPHA
            PARAM_LIST
