use crate::parser::*;
use crate::Config;

#[test]
fn program_with_newlines() {
    let input = r#"

main = -> 1


test = -> 2


"#;

    assert!(parse_string(input, &Config::default()).is_ok());
}
