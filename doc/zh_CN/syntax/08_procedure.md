# 程序

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/08_procedure.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/08_procedure.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

程序是指允许[副作用](/07_side_effect.md)的函数
基本用法或定义请参考[Function](/04_function.md)
添加 `!` 到函数名来定义它

```python
proc!(x: Int!, y: Int!) =
    for! 0..x, i =>
        for 0..y, j =>
            print! i, j
```

在处理可变对象时过程是必需的，但是将可变对象作为参数并不一定使它成为过程
这是一个函数接受一个可变对象（不是过程）

```python
peek_str s: Str! = log s

make_proc(x!: (Int => Int)): (Int => Int) = y => x! y
p! = make_proc(x => x)
print! p! 1 # 1
```

此外，过程和函数通过`proc :> func`关联
因此，可以在过程中定义函数
但是，请注意，反过来是不可能的

```python
proc!(x: Int!) = y -> log x, y # OK
func(x: Int) = y => print! x, y # NG
```

<p align='center'>
    <a href='./07_side_effect.md'>上一页</a> | <a href='./09_builtin_procs.md'>下一页</a>
</p>
