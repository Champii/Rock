


#[test]
fn testcases_basic_2_arg_fn() {
    use super::Config;

    let input = include_str!("./testcases/basic/2_arg_fn.rk");
    let expected_output = include_str!("./testcases/basic/2_arg_fn.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/2_arg_fn.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_fn_arg() {
    use super::Config;

    let input = include_str!("./testcases/basic/fn_arg.rk");
    let expected_output = include_str!("./testcases/basic/fn_arg.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/fn_arg.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_bool() {
    use super::Config;

    let input = include_str!("./testcases/basic/bool.rk");
    let expected_output = include_str!("./testcases/basic/bool.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/bool.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_main() {
    use super::Config;

    let input = include_str!("./testcases/basic/main.rk");
    let expected_output = include_str!("./testcases/basic/main.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/main.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_operator_precedence() {
    use super::Config;

    let input = include_str!("./testcases/basic/operator_precedence.rk");
    let expected_output = include_str!("./testcases/basic/operator_precedence.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/operator_precedence.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_op_func() {
    use super::Config;

    let input = include_str!("./testcases/basic/op_func.rk");
    let expected_output = include_str!("./testcases/basic/op_func.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/op_func.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_0_arg_fn() {
    use super::Config;

    let input = include_str!("./testcases/basic/0_arg_fn.rk");
    let expected_output = include_str!("./testcases/basic/0_arg_fn.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/0_arg_fn.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_basic_1_arg_fn() {
    use super::Config;

    let input = include_str!("./testcases/basic/1_arg_fn.rk");
    let expected_output = include_str!("./testcases/basic/1_arg_fn.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/basic/1_arg_fn.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}

#[test]
fn testcases_mods_main() {
    use super::Config;

    let input = include_str!("./testcases/mods/main.rk");
    let expected_output = include_str!("./testcases/mods/main.rk.out").parse::<i64>().unwrap();

    let config = Config::default();

    let actual_output = super::test::run("./testcases/mods/main.rk", input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}
