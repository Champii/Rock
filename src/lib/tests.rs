use std::path::PathBuf;

#[allow(dead_code)]
fn run(path: &str, input: &str, expected_output: &str) {
    let mut config = super::Config::default();

    config.project_config.entry_point = PathBuf::from(path);

    let expected_output = expected_output.parse::<i64>().unwrap();

    let actual_output = super::test::run(path, input.to_string(), config.clone());

    assert_eq!(expected_output, actual_output);
}
#[test]
fn testcases_trait_late_resolution_main() {
    run("./testcases/trait/late_resolution/main.rk", include_str!("./testcases/trait/late_resolution/main.rk"), include_str!("./testcases/trait/late_resolution/main.rk.out"));
}
#[test]
fn testcases_trait_multi_resolution_main() {
    run("./testcases/trait/multi_resolution/main.rk", include_str!("./testcases/trait/multi_resolution/main.rk"), include_str!("./testcases/trait/multi_resolution/main.rk.out"));
}
#[test]
fn testcases_mods_unused_impl_fn_main() {
    run("./testcases/mods/unused_impl_fn/main.rk", include_str!("./testcases/mods/unused_impl_fn/main.rk"), include_str!("./testcases/mods/unused_impl_fn/main.rk.out"));
}
#[test]
fn testcases_mods_unused_fn_main() {
    run("./testcases/mods/unused_fn/main.rk", include_str!("./testcases/mods/unused_fn/main.rk"), include_str!("./testcases/mods/unused_fn/main.rk.out"));
}
#[test]
fn testcases_mods_nested_trait_resolution_main() {
    run("./testcases/mods/nested_trait_resolution/main.rk", include_str!("./testcases/mods/nested_trait_resolution/main.rk"), include_str!("./testcases/mods/nested_trait_resolution/main.rk.out"));
}
#[test]
fn testcases_mods_func_arg_resolution_main() {
    run("./testcases/mods/func_arg_resolution/main.rk", include_str!("./testcases/mods/func_arg_resolution/main.rk"), include_str!("./testcases/mods/func_arg_resolution/main.rk.out"));
}
#[test]
fn testcases_mods_basic_mod_main() {
    run("./testcases/mods/basic_mod/main.rk", include_str!("./testcases/mods/basic_mod/main.rk"), include_str!("./testcases/mods/basic_mod/main.rk.out"));
}
#[test]
fn testcases_basic_bool_false() {
    run("./testcases/basic/bool_false.rk", include_str!("./testcases/basic/bool_false.rk"), include_str!("./testcases/basic/bool_false.rk.out"));
}
#[test]
fn testcases_basic_extern() {
    run("./testcases/basic/extern.rk", include_str!("./testcases/basic/extern.rk"), include_str!("./testcases/basic/extern.rk.out"));
}
#[test]
fn testcases_basic_main() {
    run("./testcases/basic/main.rk", include_str!("./testcases/basic/main.rk"), include_str!("./testcases/basic/main.rk.out"));
}
#[test]
fn testcases_basic_let() {
    run("./testcases/basic/let.rk", include_str!("./testcases/basic/let.rk"), include_str!("./testcases/basic/let.rk.out"));
}
#[test]
fn testcases_basic_op_func() {
    run("./testcases/basic/op_func.rk", include_str!("./testcases/basic/op_func.rk"), include_str!("./testcases/basic/op_func.rk.out"));
}
#[test]
fn testcases_basic_0_arg_fn() {
    run("./testcases/basic/0_arg_fn.rk", include_str!("./testcases/basic/0_arg_fn.rk"), include_str!("./testcases/basic/0_arg_fn.rk.out"));
}
#[test]
fn testcases_basic_operator_precedence() {
    run("./testcases/basic/operator_precedence.rk", include_str!("./testcases/basic/operator_precedence.rk"), include_str!("./testcases/basic/operator_precedence.rk.out"));
}
#[test]
fn testcases_basic_1_arg_fn() {
    run("./testcases/basic/1_arg_fn.rk", include_str!("./testcases/basic/1_arg_fn.rk"), include_str!("./testcases/basic/1_arg_fn.rk.out"));
}
#[test]
fn testcases_basic_2_arg_fn() {
    run("./testcases/basic/2_arg_fn.rk", include_str!("./testcases/basic/2_arg_fn.rk"), include_str!("./testcases/basic/2_arg_fn.rk.out"));
}
#[test]
fn testcases_basic_if_else() {
    run("./testcases/basic/if_else.rk", include_str!("./testcases/basic/if_else.rk"), include_str!("./testcases/basic/if_else.rk.out"));
}
#[test]
fn testcases_basic_recur() {
    run("./testcases/basic/recur.rk", include_str!("./testcases/basic/recur.rk"), include_str!("./testcases/basic/recur.rk.out"));
}
#[test]
fn testcases_basic_fn_arg() {
    run("./testcases/basic/fn_arg.rk", include_str!("./testcases/basic/fn_arg.rk"), include_str!("./testcases/basic/fn_arg.rk.out"));
}
#[test]
fn testcases_basic_bool_true() {
    run("./testcases/basic/bool_true.rk", include_str!("./testcases/basic/bool_true.rk"), include_str!("./testcases/basic/bool_true.rk.out"));
}
