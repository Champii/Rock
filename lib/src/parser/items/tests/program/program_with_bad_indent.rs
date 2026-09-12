use crate::parser::*;
use crate::Config;

#[test]
fn program_with_bad_indent() {
    let input = r#"main = ->
a
  2"#;

    assert!(parse_string(input, &Config::default()).is_err());
}
