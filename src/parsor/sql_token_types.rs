#[derive(Debug)]
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
}
