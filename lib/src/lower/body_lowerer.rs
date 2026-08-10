use crate::ast;
use crate::collect::item_index::ModuleKind;
use crate::ids::ModuleId;
use crate::lower::module_context::ModuleLoweringContext;
use crate::lower::Lowerer;

pub(crate) struct BodyLowerer<'a> {
    lowerer: &'a mut Lowerer,
}

impl<'a> BodyLowerer<'a> {
    pub(crate) fn new(lowerer: &'a mut Lowerer) -> Self {
        Self { lowerer }
    }

    pub(crate) fn lower_root_and_loaded_modules(&mut self, root_module: &ast::Module) {
        let Some(root_module_id) = self
            .lowerer
            .item_index
            .modules()
            .iter()
            .find(|module| module.kind == ModuleKind::Root)
            .map(|module| module.module_id)
        else {
            self.lowerer
                .diagnostics
                .push("missing indexed root module while lowering bodies".to_string());
            return;
        };
        self.lower_module(root_module, root_module_id);
        self.lower_loaded_modules();
    }

    pub(crate) fn lower_module(&mut self, module: &ast::Module, module_id: ModuleId) {
        self.lower_module_qualified(module, None, module_id);
    }

    pub(crate) fn lower_module_qualified(
        &mut self,
        module: &ast::Module,
        module_prefix: Option<&str>,
        module_id: ModuleId,
    ) {
        self.lowerer
            .lower_module_bodies_qualified_impl(module, module_prefix, module_id);
    }

    pub(crate) fn lower_loaded_modules(&mut self) {
        ModuleLoweringContext::for_each_loaded_module(
            self.lowerer,
            |lowerer, module_id, module_name, loaded_module| {
                lowerer.lower_module_bodies_qualified_impl(
                    loaded_module,
                    Some(module_name),
                    module_id,
                );
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::Program;
    use crate::crate_system::CrateContext;
    use crate::lower::Lowerer;
    use crate::source_loader::SourceDatabase;

    #[test]
    fn body_lowerer_attaches_bodies_by_source_id_despite_resolver_name_changes() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rock_body_lowerer_modules_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let entry = temp_dir.join("main.rk");
        let left = temp_dir.join("left.rk");
        let right = temp_dir.join("right.rk");
        std::fs::write(&entry, "mod left\nmod right\nmain: I64\nmain = ->\n    0\n").unwrap();
        std::fs::write(&left, "answer: I64\nanswer = ->\n    7\n< answer\n").unwrap();
        std::fs::write(&right, "answer: I64\nanswer = ->\n    8\n< answer\n").unwrap();

        let config = crate::Config {
            entry_file: entry,
            no_std: true,
            no_prelude: true,
            current_crate_name: Some("demo".to_string()),
            ..crate::Config::default()
        };
        let mut db = SourceDatabase::new();
        let graph = db.load_entry(config.entry_file.clone(), &config).unwrap();
        let left_inline_body = crate::parser::parse_string(
            "answer: I64\nanswer = ->\n    3\n",
            &crate::Config::default(),
        )
        .unwrap()
        .module;
        let right_inline_body = crate::parser::parse_string(
            "answer: I64\nanswer = ->\n    4\n",
            &crate::Config::default(),
        )
        .unwrap()
        .module;
        let mut root_module = graph.root_module().clone();
        root_module
            .top_levels
            .insert(0, ast::TopLevel::InfixOperator(5, "+".to_string()));
        root_module.top_levels.insert(
            1,
            ast::TopLevel::Module(ast::ModuleDecl(ast::Module {
                name: Some(ast::Ident {
                    name: "left_inline".to_string(),
                    span: Default::default(),
                }),
                top_levels: vec![ast::TopLevel::Module(ast::ModuleDecl(ast::Module {
                    name: Some(ast::Ident {
                        name: "shared".to_string(),
                        span: Default::default(),
                    }),
                    top_levels: left_inline_body.top_levels,
                    is_inline: true,
                    filepath: None,
                }))],
                is_inline: true,
                filepath: None,
            })),
        );
        root_module.top_levels.insert(
            2,
            ast::TopLevel::Module(ast::ModuleDecl(ast::Module {
                name: Some(ast::Ident {
                    name: "right_inline".to_string(),
                    span: Default::default(),
                }),
                top_levels: vec![ast::TopLevel::Module(ast::ModuleDecl(ast::Module {
                    name: Some(ast::Ident {
                        name: "shared".to_string(),
                        span: Default::default(),
                    }),
                    top_levels: right_inline_body.top_levels,
                    is_inline: true,
                    filepath: None,
                }))],
                is_inline: true,
                filepath: None,
            })),
        );
        let program = Program {
            module: root_module,
        };
        let crate_ctx = CrateContext::new();
        let decls = crate::collect::collect_with_source_graph(
            &program,
            &graph,
            &crate_ctx,
            false,
            Some("demo"),
        )
        .unwrap();
        let mut lowerer = Lowerer::from_declarations(decls).unwrap();
        lowerer
            .modules
            .configure_current_crate(Some("demo"), Some(&config.entry_file));
        lowerer
            .modules
            .associate_current_crate_module_ids(&lowerer.item_index);

        let main_id = lowerer.resolve_item_def_id("main").unwrap();
        let left_inline_answer_id = lowerer
            .resolve_item_def_id("left_inline::shared::answer")
            .unwrap();
        let right_inline_answer_id = lowerer
            .resolve_item_def_id("right_inline::shared::answer")
            .unwrap();
        let left_answer_id = lowerer.resolve_item_def_id("demo::left::answer").unwrap();
        let right_answer_id = lowerer.resolve_item_def_id("demo::right::answer").unwrap();
        let root_module_id = lowerer
            .item_index
            .modules()
            .iter()
            .find(|module| module.kind == ModuleKind::Root)
            .unwrap()
            .module_id;
        let left_inline_id = lowerer
            .item_index
            .child_module_id(root_module_id, "left_inline")
            .unwrap();
        let right_inline_id = lowerer
            .item_index
            .child_module_id(root_module_id, "right_inline")
            .unwrap();
        assert_ne!(
            lowerer.item_index.child_module_id(left_inline_id, "shared"),
            lowerer
                .item_index
                .child_module_id(right_inline_id, "shared"),
            "same child name under different inline parents must retain distinct ModuleIds"
        );
        assert_eq!(
            lowerer
                .item_index
                .item_at_source(
                    root_module_id,
                    program
                        .module
                        .top_levels
                        .iter()
                        .position(|top_level| {
                            matches!(top_level, ast::TopLevel::FunctionDecl(function) if function.name.name == "main")
                        })
                        .unwrap(),
                )
                .unwrap()
                .def_id,
            main_id,
            "root function should retain its source-indexed DefId"
        );
        for id in [
            main_id,
            left_inline_answer_id,
            right_inline_answer_id,
            left_answer_id,
            right_answer_id,
        ] {
            let body = &mut lowerer.items.function_mut(id).unwrap().body;
            body.stmts = vec![crate::hir::HirStmt::Continue];
            body.ty = crate::types::Type::Str;
        }
        lowerer.resolver.item_paths.clear();
        lowerer.resolver.item_names_by_id.clear();
        lowerer.resolver.import_aliases.clear();
        lowerer.resolver.export_aliases.clear();
        lowerer.resolver.scoped_module_aliases.clear();
        lowerer.resolver.module_aliases.clear();

        BodyLowerer::new(&mut lowerer).lower_root_and_loaded_modules(&program.module);

        assert!(
            lowerer
                .items
                .function(main_id)
                .unwrap()
                .body
                .stmts
                .iter()
                .all(|stmt| !matches!(stmt, crate::hir::HirStmt::Continue),),
            "root body should be lowered: {:?}",
            lowerer.errors()
        );
        assert!(
            lowerer
                .items
                .function(left_inline_answer_id)
                .unwrap()
                .body
                .stmts
                .iter()
                .all(|stmt| !matches!(stmt, crate::hir::HirStmt::Continue)),
            "left nested inline module body should be lowered"
        );
        assert!(
            lowerer
                .items
                .function(right_inline_answer_id)
                .unwrap()
                .body
                .stmts
                .iter()
                .all(|stmt| !matches!(stmt, crate::hir::HirStmt::Continue)),
            "right nested inline module body should be lowered"
        );
        assert!(
            lowerer
                .items
                .function(left_answer_id)
                .unwrap()
                .body
                .stmts
                .iter()
                .all(|stmt| !matches!(stmt, crate::hir::HirStmt::Continue)),
            "left source-backed body should be lowered"
        );
        assert!(
            lowerer
                .items
                .function(right_answer_id)
                .unwrap()
                .body
                .stmts
                .iter()
                .all(|stmt| !matches!(stmt, crate::hir::HirStmt::Continue)),
            "right source-backed body should be lowered"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn body_lowerer_lowers_source_backed_body_diagnostic_once() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rock_body_lowerer_duplicate_diagnostic_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let entry = temp_dir.join("main.rk");
        let helper = temp_dir.join("helper.rk");
        std::fs::write(&entry, "mod helper\nmain: I64\nmain = ->\n    0\n").unwrap();
        std::fs::write(&helper, "bad: I64\nbad = ->\n    \"not an integer\"\n").unwrap();

        let config = crate::Config {
            entry_file: entry.clone(),
            no_std: true,
            no_prelude: true,
            current_crate_name: Some("demo".to_string()),
            ..crate::Config::default()
        };
        let mut db = SourceDatabase::new();
        let graph = db.load_entry(entry.clone(), &config).unwrap();
        let program = Program {
            module: graph.root_module().clone(),
        };
        let crate_ctx = CrateContext::new();
        let decls = crate::collect::collect_with_source_graph(
            &program,
            &graph,
            &crate_ctx,
            false,
            Some("demo"),
        )
        .unwrap();
        let mut lowerer = Lowerer::from_declarations(decls).unwrap();
        lowerer
            .modules
            .configure_current_crate(Some("demo"), Some(&entry));
        lowerer
            .modules
            .associate_current_crate_module_ids(&lowerer.item_index);

        BodyLowerer::new(&mut lowerer).lower_root_and_loaded_modules(&program.module);

        let diagnostics = lowerer
            .errors()
            .iter()
            .filter(|error| {
                error
                    .message
                    .contains("In function 'demo::helper::bad': return type mismatch")
            })
            .count();
        assert_eq!(
            diagnostics, 1,
            "source-backed function body should be lowered exactly once"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
