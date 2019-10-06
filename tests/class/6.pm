class Foo
    bar :: Int
    def: 32
    f -> this.bar + this.def

class Bar
    foo :: Foo

main ->
    a = Foo
        bar: 10

    b = Bar
        foo: a

    b.foo.f()
