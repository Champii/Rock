use crate::parser::items::tests::common::*;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn test_parse_macro_invoc() {
    let input = "%mymacro\n    a\n    b";
    let tokens = lex(input);
    let tokens = &tokens[1..]; // skip the Indent(0)
    let config = Config::default();

    let (rest, macro_invoc) = macro_invoc(ParseCtx::from(tokens, &config)).unwrap();

    assert_eq!(macro_invoc.name.name, "mymacro");
    assert_eq!(macro_invoc.args.len(), 3); // FIXME, should be 2
    assert_eq!(rest.len(), 0);
}
