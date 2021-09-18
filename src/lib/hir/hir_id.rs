use colored::*;
use paste::paste;

macro_rules! def_id {
    ($name:ident) => {
        paste! {
            #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
            pub struct $name(pub u64);

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(
                        f,
                        "{}{}{}{}",
                        "HirId".yellow(),
                        "(".green(),
                        self.0.to_string().blue(),
                        ")".green(),
                    )
                }
            }

        }
    };
}

def_id!(HirId);
def_id!(FnBodyId);
