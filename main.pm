class Foo
    bar :: Int
    def: 24

class Bar
    foo :: Foo
    val :: Int
    f b -> b = 42

main ->
    a = Foo
        bar: 10

    b = Bar
        foo: a
        val: 8

    b.foo.def = 46
