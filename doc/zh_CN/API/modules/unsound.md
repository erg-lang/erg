# module `unsound`

提供的api执行在Erg的类型系统中无法保证安全的不健全和不安全的操作。

## `unsafe!`

执行一个`Unsafe`过程。就像Rust一样，`Unsafe`的api不能被直接调用，而是全部作为高阶函数传递给这个过程。

``` erg
unsound = import "unsound"

i = unsound.unsafe!do !:
# convert `Result Int` to `Int`
unsound.transmute input !() . try into (int), int
```

## transmute

将第一个参数的对象转换成第二个参数的类型。不进行模板检查。
这个函数损害型系统的型安全性。使用的时候请进行验证等。

## auto_transmute

与`transmute`不同，自动转换为期待的类型。与Ocaml的`Obj.magic`作用相同。