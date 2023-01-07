# 测试

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/test.md%26commit_hash%3D3e4251b9f9929891dd8ce422c1ed6853f77ab432)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/test.md&commit_hash=3e4251b9f9929891dd8ce422c1ed6853f77ab432)

测试是确保代码质量的重要部分

使用以下命令执行测试

``` sh
cargo test --features large_thread
```

由于cargo需要一个小线程来运行测试，我们使用 `large_thread` 标志来避免堆栈溢出

## 放置测试

根据实现的特性来安排它们。将解析器测试放置在`erg_parser/tests`下，将编译器(类型检查器等)测试放置在`erg_compiler/tests`下，将用户可以直接使用的语言特性测试放置在`erg/tests`下(然而，这些测试目前正在开发中，不一定按照这种惯例安排)

## 如何编写测试

有两种类型的测试。positive测试和negative测试。
positive测试是检查编译器是否按预期运行的测试，而negative测试是检查编译器是否正确地输出无效输入的错误。
由于编程语言处理器的性质，在所有软件中，它们特别容易受到无效输入的影响，并且必须始终将错误呈现给用户，因此后者也必须得到照顾。

如果你在语言中添加了一个新特性，你至少需要写一个positive测试。另外，如果可能的话，请写同时编写negative测试。
