use paste::paste;
use std::sync::atomic::{AtomicU64, Ordering};

macro_rules! def_id {
    ($name:ident) => {
        paste! {
            #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
            pub struct $name(u64);

            static [<GLOBAL_NEXT_ $name:upper>]: AtomicU64 = AtomicU64::new(0);

            impl $name {
                pub fn next() -> Self {

                    Self(AtomicU64::fetch_add(
                        &[<GLOBAL_NEXT_ $name:upper>],
                        1,
                        Ordering::SeqCst,
                    ))
                }
            }
        }
    };
}

def_id!(HirId);
def_id!(FnBodyId);
