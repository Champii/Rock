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
fn testcases_basic_main() {
    run("./testcases/basic/main.rk", include_str!("./testcases/basic/main.rk"), include_str!("./testcases/basic/main.rk.out"));
}
