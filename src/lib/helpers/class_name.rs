pub trait HasName {
    fn get_name(&self) -> String;
}

#[macro_export]
macro_rules! generate_has_name {
    ($class:tt) => {
        impl HasName for $class {
            fn get_name(&self) -> String {
                self.name.clone().to_string()
            }
        }
    };
}

pub trait ClassName {
    fn class_name_self(&self) -> String;
}

impl<T> ClassName for T
where
    T: core::fmt::Debug,
{
    fn class_name_self(&self) -> String {
        let name = format!("{:?}", self);

        name.chars().take_while(|c| c.is_alphanumeric()).collect()
    }
}
