pub trait HasName {
    fn get_name(&self) -> String;
}

#[macro_use]
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
    // fn class_name() -> String {
    //     let name = format!("{:?}", T::default());

    //     let names = name.split::<_>(" ").collect::<Vec<&str>>();
    //     let name = names.get(0).unwrap();

    //     name.to_string()
    // }

    fn class_name_self(&self) -> String {
        // T::class_name()
        let name = format!("{:?}", self);

        let names = name.split::<_>(" ").collect::<Vec<&str>>();
        let name = names.get(0).unwrap();

        name.to_string()
    }
}

#[macro_export]
macro_rules! walk_list {
    ($visitor: expr, $method: ident, $list: expr) => {
        for elem in $list {
            $visitor.$method(elem)
        }
    };

    ($visitor: expr, $method: ident, $list: expr, $($extra_args: expr),*) => {
        for elem in $list {
            $visitor.$method(elem, $($extra_args,)*)
        }
    }
}

#[macro_export]
macro_rules! walk_map {
    ($visitor: expr, $method: ident, $list: expr) => {
        for (_, elem) in $list {
            $visitor.$method(elem)
        }
    };

    ($visitor: expr, $method: ident, $list: expr, $($extra_args: expr),*) => {
        for (_, elem) in $list {
            $visitor.$method(elem, $($extra_args,)*)
        }
    }
}
