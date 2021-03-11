


#[test]
fn testcases_basic_main() {
    use super::Config;

    let input = include_str!("./testcases/basic_main.rk");
    let expected_output = 42;

    let config = Config::default();

    let actual_output = super::run(input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}
