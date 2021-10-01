use std::path::PathBuf;

#[allow(dead_code)]
fn run(path: &str, input: &str, expected_ret: &str, expected_output: &str) {
    let mut config = super::Config::default();

    config.project_config.entry_point = PathBuf::from(path);

    let expected_ret = expected_ret.parse::<i64>().unwrap();

    let (ret_code, stdout) = super::test::run(path, input.to_string(), config);

    assert_eq!(expected_ret, ret_code);
    assert_eq!(expected_output, stdout);
}
#[test]
fn testcases_trait_late_resolution_main() {
    run("testcases/trait/late_resolution/main.rk", include_str!("testcases/trait/late_resolution/main.rk"), include_str!("testcases/trait/late_resolution/main.rk.out"), include_str!("testcases/trait/late_resolution/main.rk.stdout"));
}
#[test]
fn testcases_trait_multi_resolution_main() {
    run("testcases/trait/multi_resolution/main.rk", include_str!("testcases/trait/multi_resolution/main.rk"), include_str!("testcases/trait/multi_resolution/main.rk.out"), include_str!("testcases/trait/multi_resolution/main.rk.stdout"));
}
#[test]
fn testcases_fails_basic_fn_bad_arg_nb2_main() {
    run("testcases/fails/basic/fn_bad_arg_nb2/main.rk", include_str!("testcases/fails/basic/fn_bad_arg_nb2/main.rk"), include_str!("testcases/fails/basic/fn_bad_arg_nb2/main.rk.out"), include_str!("testcases/fails/basic/fn_bad_arg_nb2/main.rk.stdout"));
}
#[test]
fn testcases_fails_basic_fn_bad_arg_nb_main() {
    run("testcases/fails/basic/fn_bad_arg_nb/main.rk", include_str!("testcases/fails/basic/fn_bad_arg_nb/main.rk"), include_str!("testcases/fails/basic/fn_bad_arg_nb/main.rk.out"), include_str!("testcases/fails/basic/fn_bad_arg_nb/main.rk.stdout"));
}
#[test]
fn testcases_fails_basic_struct_bad_field_type_main() {
    run("testcases/fails/basic/struct_bad_field_type/main.rk", include_str!("testcases/fails/basic/struct_bad_field_type/main.rk"), include_str!("testcases/fails/basic/struct_bad_field_type/main.rk.out"), include_str!("testcases/fails/basic/struct_bad_field_type/main.rk.stdout"));
}
#[test]
fn testcases_fails_basic_fn_bad_arg_main() {
    run("testcases/fails/basic/fn_bad_arg/main.rk", include_str!("testcases/fails/basic/fn_bad_arg/main.rk"), include_str!("testcases/fails/basic/fn_bad_arg/main.rk.out"), include_str!("testcases/fails/basic/fn_bad_arg/main.rk.stdout"));
}
#[test]
fn testcases_mods_unused_impl_fn_main() {
    run("testcases/mods/unused_impl_fn/main.rk", include_str!("testcases/mods/unused_impl_fn/main.rk"), include_str!("testcases/mods/unused_impl_fn/main.rk.out"), include_str!("testcases/mods/unused_impl_fn/main.rk.stdout"));
}
#[test]
fn testcases_mods_unused_fn_main() {
    run("testcases/mods/unused_fn/main.rk", include_str!("testcases/mods/unused_fn/main.rk"), include_str!("testcases/mods/unused_fn/main.rk.out"), include_str!("testcases/mods/unused_fn/main.rk.stdout"));
}
#[test]
fn testcases_mods_nested_trait_resolution_main() {
    run("testcases/mods/nested_trait_resolution/main.rk", include_str!("testcases/mods/nested_trait_resolution/main.rk"), include_str!("testcases/mods/nested_trait_resolution/main.rk.out"), include_str!("testcases/mods/nested_trait_resolution/main.rk.stdout"));
}
#[test]
fn testcases_mods_full_fact_main() {
    run("testcases/mods/full_fact/main.rk", include_str!("testcases/mods/full_fact/main.rk"), include_str!("testcases/mods/full_fact/main.rk.out"), include_str!("testcases/mods/full_fact/main.rk.stdout"));
}
#[test]
fn testcases_mods_func_arg_resolution_main() {
    run("testcases/mods/func_arg_resolution/main.rk", include_str!("testcases/mods/func_arg_resolution/main.rk"), include_str!("testcases/mods/func_arg_resolution/main.rk.out"), include_str!("testcases/mods/func_arg_resolution/main.rk.stdout"));
}
#[test]
fn testcases_mods_basic_mod_main() {
    run("testcases/mods/basic_mod/main.rk", include_str!("testcases/mods/basic_mod/main.rk"), include_str!("testcases/mods/basic_mod/main.rk.out"), include_str!("testcases/mods/basic_mod/main.rk.stdout"));
}
#[test]
fn testcases_basic_bool_true_main() {
    run("testcases/basic/bool_true/main.rk", include_str!("testcases/basic/bool_true/main.rk"), include_str!("testcases/basic/bool_true/main.rk.out"), include_str!("testcases/basic/bool_true/main.rk.stdout"));
}
#[test]
fn testcases_basic_op_func_main() {
    run("testcases/basic/op_func/main.rk", include_str!("testcases/basic/op_func/main.rk"), include_str!("testcases/basic/op_func/main.rk.out"), include_str!("testcases/basic/op_func/main.rk.stdout"));
}
#[test]
fn testcases_basic_multiline_struct_const_main() {
    run("testcases/basic/multiline_struct_const/main.rk", include_str!("testcases/basic/multiline_struct_const/main.rk"), include_str!("testcases/basic/multiline_struct_const/main.rk.out"), include_str!("testcases/basic/multiline_struct_const/main.rk.stdout"));
}
#[test]
fn testcases_basic_2_arg_fn_main() {
    run("testcases/basic/2_arg_fn/main.rk", include_str!("testcases/basic/2_arg_fn/main.rk"), include_str!("testcases/basic/2_arg_fn/main.rk.out"), include_str!("testcases/basic/2_arg_fn/main.rk.stdout"));
}
#[test]
fn testcases_basic_monomorph_in_trait_main() {
    run("testcases/basic/monomorph_in_trait/main.rk", include_str!("testcases/basic/monomorph_in_trait/main.rk"), include_str!("testcases/basic/monomorph_in_trait/main.rk.out"), include_str!("testcases/basic/monomorph_in_trait/main.rk.stdout"));
}
#[test]
fn testcases_basic_trait_use_before_decl_main() {
    run("testcases/basic/trait_use_before_decl/main.rk", include_str!("testcases/basic/trait_use_before_decl/main.rk"), include_str!("testcases/basic/trait_use_before_decl/main.rk.out"), include_str!("testcases/basic/trait_use_before_decl/main.rk.stdout"));
}
#[test]
fn testcases_basic_nested_struct_dect_multiline_main() {
    run("testcases/basic/nested_struct_dect_multiline/main.rk", include_str!("testcases/basic/nested_struct_dect_multiline/main.rk"), include_str!("testcases/basic/nested_struct_dect_multiline/main.rk.out"), include_str!("testcases/basic/nested_struct_dect_multiline/main.rk.stdout"));
}
#[test]
fn testcases_basic_array_main() {
    run("testcases/basic/array/main.rk", include_str!("testcases/basic/array/main.rk"), include_str!("testcases/basic/array/main.rk.out"), include_str!("testcases/basic/array/main.rk.stdout"));
}
#[test]
fn testcases_basic_nested_struct_main() {
    run("testcases/basic/nested_struct/main.rk", include_str!("testcases/basic/nested_struct/main.rk"), include_str!("testcases/basic/nested_struct/main.rk.out"), include_str!("testcases/basic/nested_struct/main.rk.stdout"));
}
#[test]
fn testcases_basic_simple_struct_main() {
    run("testcases/basic/simple_struct/main.rk", include_str!("testcases/basic/simple_struct/main.rk"), include_str!("testcases/basic/simple_struct/main.rk.out"), include_str!("testcases/basic/simple_struct/main.rk.stdout"));
}
#[test]
fn testcases_basic_reassign_main() {
    run("testcases/basic/reassign/main.rk", include_str!("testcases/basic/reassign/main.rk"), include_str!("testcases/basic/reassign/main.rk.out"), include_str!("testcases/basic/reassign/main.rk.stdout"));
}
#[test]
fn testcases_basic_indice_assign_main() {
    run("testcases/basic/indice_assign/main.rk", include_str!("testcases/basic/indice_assign/main.rk"), include_str!("testcases/basic/indice_assign/main.rk.out"), include_str!("testcases/basic/indice_assign/main.rk.stdout"));
}
#[test]
fn testcases_basic_dot_assign_main() {
    run("testcases/basic/dot_assign/main.rk", include_str!("testcases/basic/dot_assign/main.rk"), include_str!("testcases/basic/dot_assign/main.rk.out"), include_str!("testcases/basic/dot_assign/main.rk.stdout"));
}
#[test]
fn testcases_basic_extern_main() {
    run("testcases/basic/extern/main.rk", include_str!("testcases/basic/extern/main.rk"), include_str!("testcases/basic/extern/main.rk.out"), include_str!("testcases/basic/extern/main.rk.stdout"));
}
#[test]
fn testcases_basic_1_arg_fn_main() {
    run("testcases/basic/1_arg_fn/main.rk", include_str!("testcases/basic/1_arg_fn/main.rk"), include_str!("testcases/basic/1_arg_fn/main.rk.out"), include_str!("testcases/basic/1_arg_fn/main.rk.stdout"));
}
#[test]
fn testcases_basic_monomorph_main() {
    run("testcases/basic/monomorph/main.rk", include_str!("testcases/basic/monomorph/main.rk"), include_str!("testcases/basic/monomorph/main.rk.out"), include_str!("testcases/basic/monomorph/main.rk.stdout"));
}
#[test]
fn testcases_basic_bool_false_main() {
    run("testcases/basic/bool_false/main.rk", include_str!("testcases/basic/bool_false/main.rk"), include_str!("testcases/basic/bool_false/main.rk.out"), include_str!("testcases/basic/bool_false/main.rk.stdout"));
}
#[test]
fn testcases_basic_struct_array_field_main() {
    run("testcases/basic/struct_array_field/main.rk", include_str!("testcases/basic/struct_array_field/main.rk"), include_str!("testcases/basic/struct_array_field/main.rk.out"));
}
#[test]
fn testcases_basic_struct_array_field_main() {
    run("testcases/basic/struct_array_field/main.rk", include_str!("testcases/basic/struct_array_field/main.rk"), include_str!("testcases/basic/struct_array_field/main.rk.out"));
}
#[test]
fn testcases_basic_struct_array_field_main() {
    run("testcases/basic/struct_array_field/main.rk", include_str!("testcases/basic/struct_array_field/main.rk"), include_str!("testcases/basic/struct_array_field/main.rk.out"));
}
#[test]
fn testcases_basic_main_main() {
    run("testcases/basic/main/main.rk", include_str!("testcases/basic/main/main.rk"), include_str!("testcases/basic/main/main.rk.out"), include_str!("testcases/basic/main/main.rk.stdout"));
}
#[test]
fn testcases_basic_fn_arg_main() {
    run("testcases/basic/fn_arg/main.rk", include_str!("testcases/basic/fn_arg/main.rk"), include_str!("testcases/basic/fn_arg/main.rk.out"), include_str!("testcases/basic/fn_arg/main.rk.stdout"));
}
#[test]
fn testcases_basic_recur_main() {
    run("testcases/basic/recur/main.rk", include_str!("testcases/basic/recur/main.rk"), include_str!("testcases/basic/recur/main.rk.out"), include_str!("testcases/basic/recur/main.rk.stdout"));
}
#[test]
fn testcases_basic_struct_index_main() {
    run("testcases/basic/struct_index/main.rk", include_str!("testcases/basic/struct_index/main.rk"), include_str!("testcases/basic/struct_index/main.rk.out"), include_str!("testcases/basic/struct_index/main.rk.stdout"));
}
#[test]
fn testcases_basic_nested_array_main() {
    run("testcases/basic/nested_array/main.rk", include_str!("testcases/basic/nested_array/main.rk"), include_str!("testcases/basic/nested_array/main.rk.out"), include_str!("testcases/basic/nested_array/main.rk.stdout"));
}
#[test]
fn testcases_basic_if_else_main() {
    run("testcases/basic/if_else/main.rk", include_str!("testcases/basic/if_else/main.rk"), include_str!("testcases/basic/if_else/main.rk.out"), include_str!("testcases/basic/if_else/main.rk.stdout"));
}
#[test]
fn testcases_basic_operator_precedence_main() {
    run("testcases/basic/operator_precedence/main.rk", include_str!("testcases/basic/operator_precedence/main.rk"), include_str!("testcases/basic/operator_precedence/main.rk.out"), include_str!("testcases/basic/operator_precedence/main.rk.stdout"));
}
#[test]
fn testcases_basic_let_main() {
    run("testcases/basic/let/main.rk", include_str!("testcases/basic/let/main.rk"), include_str!("testcases/basic/let/main.rk.out"), include_str!("testcases/basic/let/main.rk.stdout"));
}
#[test]
fn testcases_basic_fn_arg_array_main() {
    run("testcases/basic/fn_arg_array/main.rk", include_str!("testcases/basic/fn_arg_array/main.rk"), include_str!("testcases/basic/fn_arg_array/main.rk.out"), include_str!("testcases/basic/fn_arg_array/main.rk.stdout"));
}
#[test]
fn testcases_basic_0_arg_fn_main() {
    run("testcases/basic/0_arg_fn/main.rk", include_str!("testcases/basic/0_arg_fn/main.rk"), include_str!("testcases/basic/0_arg_fn/main.rk.out"), include_str!("testcases/basic/0_arg_fn/main.rk.stdout"));
}
#[test]
fn testcases_basic_trait_monomorph_main() {
    run("testcases/basic/trait_monomorph/main.rk", include_str!("testcases/basic/trait_monomorph/main.rk"), include_str!("testcases/basic/trait_monomorph/main.rk.out"), include_str!("testcases/basic/trait_monomorph/main.rk.stdout"));
}
