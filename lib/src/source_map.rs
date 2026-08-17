use std::collections::HashMap;

use crate::collect::item_index::ItemIndex;
use crate::ids::{DefId, HirLocalId};
use crate::lexer::Span;

#[derive(Debug, Clone, Default)]
pub struct SemanticSourceMap {
    definitions: HashMap<DefId, Span>,
    locals: HashMap<(DefId, HirLocalId), Span>,
}

impl SemanticSourceMap {
    pub fn from_item_index(index: &ItemIndex) -> Self {
        Self {
            definitions: index
                .items()
                .iter()
                .map(|item| (item.def_id, item.name_span.clone()))
                .collect(),
            locals: HashMap::new(),
        }
    }

    pub fn definition_span(&self, id: DefId) -> Option<&Span> {
        self.definitions.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn insert_definition(&mut self, id: DefId, span: Span) {
        self.definitions.insert(id, span);
    }

    pub fn insert_local(&mut self, owner: DefId, local: HirLocalId, span: Span) {
        self.locals.insert((owner, local), span);
    }

    pub fn local_span(&self, owner: DefId, local: HirLocalId) -> Option<&Span> {
        self.locals.get(&(owner, local))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use crate::collect::item_index::{index_root_module_items, IndexingIds};

    #[test]
    fn semantic_source_map_preserves_indexed_definition_name_span() {
        let path = PathBuf::from("/virtual/main.rk");
        let module = crate::parser::parse_source(
            path.clone(),
            "struct Widget\n\nmain = -> 0\n",
            &crate::Config::default(),
        )
        .expect("source should parse");
        let mut ids = IndexingIds::new_root();
        let index = index_root_module_items(&mut ids, &module);
        let widget = index
            .items()
            .iter()
            .find(|item| item.name == "Widget")
            .expect("Widget definition");

        let sources = SemanticSourceMap::from_item_index(&index);
        let span = sources
            .definition_span(widget.def_id)
            .expect("definition span");
        assert_eq!(span.file_path, path);
        assert_eq!(
            &"struct Widget\n\nmain = -> 0\n"[span.start..span.end],
            "Widget"
        );
    }
}
