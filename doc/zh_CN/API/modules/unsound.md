# 模块 `unsound`

让 API 执行在 Erg 的类型系统中无法保证的不健全和不安全的操作。

## `unsafe!`

执行“不安全”过程。 就像 Rust 一样，`Unsafe` API 不能直接调用，而是作为高阶函数传递给这个过程。

``` erg
unsound = import "unsound"

i = unsound. unsafe! do!:
     # 将 `Result Int` 转换为 `Int`
     unsound.transmute input!().try_into(Int), Int
```

## transmit

将第一个参数的对象转换为第二个参数的类型。没有进行类型检查。
这个函数破坏了类型系统的类型安全。请在使用前进行验证。

## 隐式转换

与 `transmute` 不同，它会自动转换为预期的类型。与 Ocaml 的 `Obj.magic` 工作方式相同。