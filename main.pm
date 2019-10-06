class Foo
    bar :: Int

class Bar
    foo :: Foo
    val :: Int
    f -> @foo.bar = 42

main ->
    a = Foo
        bar: 10

    b = Bar
        foo: a
        val: 8

    b.f()

    b.foo.bar
