#[test]
pub fn simple() {
    let expected = vec![
        ("main :: Int -> 1", 1),
        ("main :: Int -> 1 + 2", 3),
        (
            "main :: Int -> 
    1",
            1,
        ),
        (
            "main :: Int -> 
    1
",
            1,
        ),
    ];

    for exp in expected {
        let res = super::run_str(exp.0.to_string(), "main\0".to_string()).unwrap();

        assert_eq!(res, exp.1);
    }
}

#[test]
pub fn functions() {
    let expected = vec![
        (
            "one :: Int -> 1
main :: Int -> one()",
            1,
        ),
        (
            "id(a: Int) :: Int -> a
main :: Int -> id(2)",
            2,
        ),
        (
            "add(a: Int, b: Int) :: Int -> a + b
main :: Int -> add(2, 3)",
            5,
        ),
        (
            "main :: Int ->
    1
    2

    3

",
            3,
        ),
    ];

    for exp in expected {
        let res = super::run_str(exp.0.to_string(), "main\0".to_string()).unwrap();

        assert_eq!(res, exp.1);
    }
}

#[test]
pub fn variables() {
    let expected = vec![
        (
            "main :: Int ->
    a:Int = 1
    b:Int = 2
",
            2,
        ),
        (
            "main :: Int ->
    a: Int = 1
    b: Int = 2
    c: Int = a + b
",
            3,
        ),
        (
            "main :: Int ->
    a: Int = 1
    b: Int = 2
    c: Int = a + b
    c
",
            3,
        ),
        (
            "add(a:Int, b: Int) :: Int -> a + b
main :: Int ->
    a: Int = 1
    b: Int = 2
    c: Int = add(a, b)
    c
",
            3,
        ),
        //         (
        //             "main :: Int ->
        //     a: Int = 1
        //     a = 2
        //     a
        // ",
        //             2,
        //         ),
    ];

    for exp in expected {
        let res = super::run_str(exp.0.to_string(), "main\0".to_string()).unwrap();

        assert_eq!(res, exp.1);
    }
}

#[test]
pub fn if_else() {
    let expected = vec![
        (
            "main :: Int ->
    if false
        1
    else
        2
    3
",
            3,
        ),
        (
            "main :: Int ->
    if true => 1
    else 2
    3
",
            3,
        ),
        (
            "main :: Int ->
    a: Int = 0
    if true => a = 1
    else a = 2
    a
",
            1,
        ),
    ];

    for exp in expected {
        let res = super::run_str(exp.0.to_string(), "main\0".to_string()).unwrap();

        assert_eq!(res, exp.1);
    }
}

#[test]
pub fn inference() {
    let expected = vec![
        ("main -> 1", 1),
        ("main -> 1 + 1", 2),
        (
            "add a, b -> a + b
main -> add 1, 2",
            3,
        ),
        (
            "add a, b -> a + b
main -> 
    a = 1
    b = 2
    add a, b",
            3,
        ),
    ];

    for exp in expected {
        let res = super::run_str(exp.0.to_string(), "main\0".to_string()).unwrap();

        assert_eq!(res, exp.1);
    }
}
