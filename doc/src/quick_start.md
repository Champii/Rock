# Quick start

The actual best way to start is to build and install Rock via cargo:

```sh
cargo install --git https://github.com/Champii/Rock --locked
rock -V
```
Then to create a new empty project folder

```sh
rock new my_project
cd my_project
```

The project folder should contain a `src` folder with a `src/main.rk` file that should look like this:

```haskell
main: -> "Hello World !".print!
```

You can immediately build and run this default snippet with

```sh
rock run
```

This should output

```sh
Hello World !
```

The compiler has created a `build` folder containing your compiled executable `build/a.out`

