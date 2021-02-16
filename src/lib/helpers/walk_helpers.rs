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
