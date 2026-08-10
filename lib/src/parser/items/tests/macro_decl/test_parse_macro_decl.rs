use crate::parser::items::tests::common::*;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn test_parse_macro_decl() {
    let input = "macro mymacro\n    $a:ident, $b:ty =>\n        statement";
    let tokens = lex(input);
    let tokens = &tokens[1..]; // skip the Indent(0)
    let config = Config::default();

    let (rest, macro_decl) = macro_decl(ParseCtx::from(tokens, &config)).unwrap();

    assert_eq!(macro_decl.name.name, "mymacro");
    assert_eq!(macro_decl.entries.len(), 1);
    assert_eq!(rest.len(), 0);
}
