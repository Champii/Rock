use rock_lib::ast::TopLevel;
use rock_lib::parser::parse_string;
use rock_lib::Config;

// This lives as a top-level integration test so the exact command
// `cargo test -p rock-lib test_parse_struct_with_fields -- --exact --nocapture`
// runs a real parser coverage test.
#[test]
fn test_parse_struct_with_fields() {
    let program = parse_string(
        "struct Test\n    field: Type\n    < field2: Type2\n",
        &Config::default(),
    )
    .unwrap();

    let struct_decl = match &program.module.top_levels[0] {
        TopLevel::StructDecl(struct_decl) => struct_decl,
        other => panic!("Expected StructDecl, got {other:?}"),
    };

    assert_eq!(struct_decl.name.name, "Test");
    assert_eq!(struct_decl.fields.len(), 2);
    assert_eq!(struct_decl.fields[0].name.name, "field");
    assert!(!struct_decl.fields[0].public);
    assert_eq!(struct_decl.fields[1].name.name, "field2");
    assert!(struct_decl.fields[1].public);
}
