# module `unsound`

Provides APIs perform unsound and unsafe operations that cannot be guaranteed safe in Erg's type system.

## `unsafe!`

Executes an `Unsafe` procedure. Just like Rust, `Unsafe` APIs cannot be called directly, but are all passed as higher-order functions to this procedure.

``` erg
unsound = import "unsound"

i = unsound. unsafe! do!:
     # convert `Result Int` to `Int`
     unsound.transmute input!().try_into(Int), Int
```

## transmit

Converts the object of the first argument to the type of the second argument. No type checking is done.
This function breaks the type safety of the type system. Please perform validation before using.

## auto_transmute

Unlike `transmute`, it automatically converts to the expected type. Works the same as Ocaml's `Obj.magic`.