use crate::parser::*;
use crate::Config;

#[test]
fn test_complex_nested_lambda() {
    let input = "main = ->\n    foo\n        .bar lol ->\n            mdr\n                .haha lol\n                    .tata\n        .lol\n";
    let config = Config::default();

    let result = parse_string(input, &config);

    if let Err(e) = &result {
        println!("Parse error: {:?}", e);
    }

    assert!(result.is_ok(), "Failed: {:?}", result.err());
}
