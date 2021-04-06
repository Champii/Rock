


#[test]
fn testcases_basic_2_arg_fn() {
    use super::Config;

    let input = include_str!("./testcases/basic/2_arg_fn.rk");
    let expected_output = 42;

    let config = Config::default();

    let actual_output = super::test::run(input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_main() {
    use super::Config;

    let input = include_str!("./testcases/basic/main.rk");
    let expected_output = 42;

    let config = Config::default();

    let actual_output = super::test::run(input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_0_arg_fn() {
    use super::Config;

    let input = include_str!("./testcases/basic/0_arg_fn.rk");
    let expected_output = 42;

    let config = Config::default();

    let actual_output = super::test::run(input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_1_arg_fn() {
    use super::Config;

    let input = include_str!("./testcases/basic/1_arg_fn.rk");
    let expected_output = 42;

    let config = Config::default();

    let actual_output = super::test::run(input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_mods_mymod() {
    use super::Config;

    let input = include_str!("./testcases/mods/mymod.rk");
    let expected_output = 42;

    let config = Config::default();

    let actual_output = super::test::run(input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_mods_main() {
    use super::Config;

    let input = include_str!("./testcases/mods/main.rk");
    let expected_output = 42;

    let config = Config::default();

    let actual_output = super::test::run(input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}
