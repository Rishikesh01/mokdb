#[derive(Debug, PartialEq)]
pub enum SQLTokenTypes {
    //Data Manipulation language
    SELECT,
    INSERT,
    DELETE,
    UPDATE,
    //Data Definiation language
    CREATE,
    DROP,
    TRUNCATE,
    RENAME,
    ALTER,
    //TCL
    COMMIT,
    ROLLBACK,
    SAVEPOINT,
    TABLE_IDENTIFIER,
    IDENTIFIER,
    NUMBER,
    EOF,

    //other
    LEFTPAREN,
    RIGHTPAREN,
    STAR,
    COMMA,
    SEMICOLON,
    NEWLINE,

    //logical
    GREATER,
    LESSER,
    EQUAL,

    PRIMARY,
    KEY,
    NOT,
    UNIQUE,
    NULL,

    INTO,
    VALUES,

    SET,
    WHERE,
    FROM,

    AND,
    OR,
    GREATER_EQUAL,
    LESSER_EQUAL,
    NOT_EQUAL,
    STRING,
    TABLE,

    IS,
    GREATER_OR_EQUAL,
    LESSER_OR_EQUAL,
}
