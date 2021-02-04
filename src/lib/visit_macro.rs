#[macro_export]
macro_rules! generate_visitor_trait {
    (
        $(
            $(struct $name_class:ty : {
                $(
                    $( [ $child_field:ident => $child_method:ty ] )?
                    $( $child:ident )?
                    $( _ )?
                ),*
            })?

            $(enum $name_enum:ty : (
                $(
                    $prop_name:ident (
                        $(
                            $( $attr:ident => $ty:ty)?
                            $( [$attr_vec:ident => $ty_vec:ty ])?
                        ),*
                    )
                ),*
            ))?
            ,
        )*
    ) => {

        pub trait Visitor<'ast>: Sized {
            fn visit_name(&mut self, _name: String) {
                // Nothing
            }

            fn visit_primitive<T>(&mut self, _val: T)
            {}

            $(
                paste::paste! {
                    // Class implem
                    $(
                        fn [<visit_ $name_class:snake>](&mut self, node: &'ast $name_class) {
                            self.[<walk_ $name_class:snake>](node);
                        }

                        fn [<walk_ $name_class:snake>](&mut self, _node: &'ast $name_class) {
                            $(
                                $(self.[<visit_ $child>](&_node.$child);)?
                                $(walk_list!(self, [<visit_ $child_method:snake>], &_node.$child_field);)?
                            )*
                        }
                    )?

                    // Enum implem
                    $(
                        fn [<visit_ $name_enum:snake>](&mut self, node: &'ast $name_enum) {
                            self.[<walk_ $name_enum:snake>](node);
                        }

                        fn [<walk_ $name_enum:snake>](&mut self, node: &'ast $name_enum) {
                            match &node.kind {
                                $(
                                    $name_enum::$prop_name(
                                        $(
                                            $($attr)?
                                            $()?
                                        )*
                                    )
                                )*
                            }
                            // $(
                            //     $(self.[<visit_ $child>](&_node.$child);)?
                            //     $(walk_list!(self, [<visit_ $child_method:snake>], &_node.$child_field);)?
                            // )*
                        }
                    )?
                }
            )*
        }
    };
}

#[derive(Debug)]
pub struct Root {
    top_levels: Vec<TopLevel>,
    body: Body,
}

#[derive(Debug)]
pub struct TopLevel {}

#[derive(Debug)]
pub struct Body {
    statement: Statement,
}

#[derive(Debug)]
pub struct Statement {
    kind: StatementKind,
}

#[derive(Debug)]
pub enum StatementKind {
    Ident(String),
    Number(i64),
    Body(Box<Body>),
    Bodies(Vec<Body>),
}

generate_visitor_trait!(
    struct Root : {[top_levels => TopLevel], body},
    struct TopLevel : {_},
    struct Body: { statement },
    enum Statement: (
        Ident(x => String),
        Body(y => Body),
        Bodies([y => Body]),
    ),
);

struct Test {}

impl<'a> Visitor<'a> for Test {
    fn visit_root(&mut self, root: &'a Root) {
        self.walk_root(root);
    }
}

pub fn lol() {
    let mut test = Test {};

    let root = Root {
        top_levels: vec![TopLevel {}],
        body: Body {
            statement: Statement {
                kind: StatementKind::Ident("".to_string()),
            },
        },
    };

    test.visit_root(&root);
}
