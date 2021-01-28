use std::sync::atomic::{AtomicU64, Ordering};

pub type CrateId = u64;
pub type DefIndex = u64;
pub type LocalId = u64;

macro_rules! def_id {
    ($name:ident) => {
        concat_idents!(glob = GLOBAL_NEXT_, $name {
            #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
            pub struct $name(u64);

            static glob: AtomicU64 = AtomicU64::new(0);

            impl $name {
                pub fn next() -> Self {
                    Self(AtomicU64::fetch_add(
                        &glob,
                        1,
                        Ordering::SeqCst,
                    ))
                }
            }
        });
    };
}

def_id!(HirId);
def_id!(BodyId);

// impl LocalId {
//     fn next() -> Self {
//         AtomicUInt64;
//     }
// }

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// pub struct DefId {
//     root: CrateId,
//     index: DefIndex,
// }

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// pub struct HirId {
//     pub owner: DefId,
//     pub local_id: LocalId,
// }

// impl HirId {
//     pub fn new(owner: DefId) -> Self {
//         Self {
//             owner,
//             local_id: LocalId::next(owner),
//         }
//     }
// }

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// pub struct BodyId {
//     pub id: u64,
// }
