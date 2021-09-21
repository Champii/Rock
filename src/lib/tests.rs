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
fn testcases_mods_full_fact_main() {
    run("./testcases/mods/full_fact/main.rk", include_str!("./testcases/mods/full_fact/main.rk"), include_str!("./testcases/mods/full_fact/main.rk.out"));
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
fn testcases_basic_bool_true_main() {
    run("./testcases/basic/bool_true/main.rk", include_str!("./testcases/basic/bool_true/main.rk"), include_str!("./testcases/basic/bool_true/main.rk.out"));
}
#[test]
fn testcases_basic_op_func_main() {
    run("./testcases/basic/op_func/main.rk", include_str!("./testcases/basic/op_func/main.rk"), include_str!("./testcases/basic/op_func/main.rk.out"));
}
#[test]
fn testcases_basic_2_arg_fn_main() {
    run("./testcases/basic/2_arg_fn/main.rk", include_str!("./testcases/basic/2_arg_fn/main.rk"), include_str!("./testcases/basic/2_arg_fn/main.rk.out"));
}
#[test]
fn testcases_basic_monomorph_in_trait_main() {
    run("./testcases/basic/monomorph_in_trait/main.rk", include_str!("./testcases/basic/monomorph_in_trait/main.rk"), include_str!("./testcases/basic/monomorph_in_trait/main.rk.out"));
}
#[test]
fn testcases_basic_extern_main() {
    run("./testcases/basic/extern/main.rk", include_str!("./testcases/basic/extern/main.rk"), include_str!("./testcases/basic/extern/main.rk.out"));
}
#[test]
fn testcases_basic_1_arg_fn_main() {
    run("./testcases/basic/1_arg_fn/main.rk", include_str!("./testcases/basic/1_arg_fn/main.rk"), include_str!("./testcases/basic/1_arg_fn/main.rk.out"));
}
#[test]
fn testcases_basic_monomorph_main() {
    run("./testcases/basic/monomorph/main.rk", include_str!("./testcases/basic/monomorph/main.rk"), include_str!("./testcases/basic/monomorph/main.rk.out"));
}
#[test]
fn testcases_basic_bool_false_main() {
    run("./testcases/basic/bool_false/main.rk", include_str!("./testcases/basic/bool_false/main.rk"), include_str!("./testcases/basic/bool_false/main.rk.out"));
}
#[test]
fn testcases_basic_main_main() {
    run("./testcases/basic/main/main.rk", include_str!("./testcases/basic/main/main.rk"), include_str!("./testcases/basic/main/main.rk.out"));
}
#[test]
fn testcases_basic_fn_arg_main() {
    run("./testcases/basic/fn_arg/main.rk", include_str!("./testcases/basic/fn_arg/main.rk"), include_str!("./testcases/basic/fn_arg/main.rk.out"));
}
#[test]
fn testcases_basic_recur_main() {
    run("./testcases/basic/recur/main.rk", include_str!("./testcases/basic/recur/main.rk"), include_str!("./testcases/basic/recur/main.rk.out"));
}
#[test]
fn testcases_basic_if_else_main() {
    run("./testcases/basic/if_else/main.rk", include_str!("./testcases/basic/if_else/main.rk"), include_str!("./testcases/basic/if_else/main.rk.out"));
}
#[test]
fn testcases_basic_operator_precedence_main() {
    run("./testcases/basic/operator_precedence/main.rk", include_str!("./testcases/basic/operator_precedence/main.rk"), include_str!("./testcases/basic/operator_precedence/main.rk.out"));
}
#[test]
fn testcases_basic_let_main() {
    run("./testcases/basic/let/main.rk", include_str!("./testcases/basic/let/main.rk"), include_str!("./testcases/basic/let/main.rk.out"));
}
#[test]
fn testcases_basic_0_arg_fn_main() {
    run("./testcases/basic/0_arg_fn/main.rk", include_str!("./testcases/basic/0_arg_fn/main.rk"), include_str!("./testcases/basic/0_arg_fn/main.rk.out"));
}
#[test]
fn testcases_basic_trait_monomorph_main() {
    run("./testcases/basic/trait_monomorph/main.rk", include_str!("./testcases/basic/trait_monomorph/main.rk"), include_str!("./testcases/basic/trait_monomorph/main.rk.out"));
}
