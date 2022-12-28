# 程序

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/08_procedure.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/08_procedure.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

程序是指允許[副作用](/07_side_effect.md)的函數
基本用法或定義請參考[Function](/04_function.md)
添加 `!` 到函數名來定義它

```python
proc!(x: Int!, y: Int!) =
    for! 0..x, i =>
        for 0..y, j =>
            print! i, j
```

在處理可變對象時過程是必需的，但是將可變對像作為參數並不一定使它成為過程
這是一個函數接受一個可變對象（不是過程）

```python
peek_str s: Str! = log s

make_proc(x!: (Int => Int)): (Int => Int) = y => x! y
p! = make_proc(x => x)
print! p! 1 # 1
```

```python
peek_str s: Str! = log s
```
此外，過程和函數通過`proc :> func`關聯
因此，可以在過程中定義函數
但是，請注意，反過來是不可能的

```python
proc!(x: Int!) = y -> log x, y # OK
func(x: Int) = y => print! x, y # NG
```

<p align='center'>
    <a href='./07_side_effect.md'>上一頁</a> | <a href='./09_builtin_procs.md'>下一頁</a>
</p>
