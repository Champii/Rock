class Foo
    bar :: Int
    def: 30
    f a -> @bar + @def + a

class Bar
    foo :: Foo

main ->
    a = Foo
        bar: 10

    b = Bar
        foo: a

    b.foo.f 2
