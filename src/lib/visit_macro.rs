use paste::paste;

macro_rules! generate_visitor_trait {
    (
        $(
            $name:ty : {
                $(
                    $( [ $child_field:ident => $child_method:ty ] )?
                    $( $child:ident )?
                    $( _ )?
                ),*
            },
        )*
    ) => {

        pub trait Visitor<'ast>: Sized {
            fn visit_name(&mut self, _name: String) {
                // Nothing
            }

            fn visit_primitive<T>(&mut self, _val: T)
            {}

            $(
                paste! {
                    fn [<visit_ $name:snake>](&mut self, node: &'ast $name) {
                        self.[<walk_ $name:snake>](node);
                    }

                    fn [<walk_ $name:snake>](&mut self, node: &'ast $name) {
                        $(
                            $(self.[<visit_ $child>](&node.$child);)?
                            $(walk_list!(self, [<visit_ $child_method:snake>], &node.$child_field);)?
                        )*
                    }

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
pub struct Body {}

generate_visitor_trait!(
    Root : {[top_levels => TopLevel], body},
    TopLevel : {_},
    Body: {_},
);

struct Test {}

impl<'a> Visitor<'a> for Test {
    fn visit_root(&mut self, root: &'a Root) {
        println!("ROOT {:#?}", root);

        self.walk_root(root);
    }
}

pub fn lol() {
    let mut test = Test {};

    let root = Root {
        top_levels: vec![TopLevel {}],
        body: Body {},
    };

    test.visit_root(&root);
}
