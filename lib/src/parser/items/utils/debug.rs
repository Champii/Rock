pub fn debug<T: std::fmt::Debug>(input: T) -> T {
    println!("{:#?}", input);
    input
}
