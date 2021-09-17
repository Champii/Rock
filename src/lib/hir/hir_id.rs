use paste::paste;


macro_rules! def_id {
    ($name:ident) => {
        paste! {
            #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
            pub struct $name(pub u64);
        }
    };
}

def_id!(HirId);
def_id!(FnBodyId);
