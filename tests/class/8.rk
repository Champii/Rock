class Foo
    bar :: Int
    def: 24

class Bar
    foo :: Foo
    val :: Int
    f -> @foo.bar + @foo.def + @val

main ->
    a = Foo
        bar: 10

    b = Bar
        foo: a
        val: 8

    b.f()
