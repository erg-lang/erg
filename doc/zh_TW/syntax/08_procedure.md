# 程序

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/08_procedure.md%26commit_hash%3D637109aa8b3826b78df334ef6508131cff575623)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/08_procedure.md&commit_hash=637109aa8b3826b78df334ef6508131cff575623)

程序是指允許[副作用](./07_side_effect.md)的函數
基本用法或定義請參考[Function](./04_function.md)
添加 `!` 到函數名來定義它

```python
proc!(x: Int!, y: Int!) =
    for! 0..x, i =>
        for 0..y, j =>
            print! i, j
```

在處理可變對象時過程是必需的，但是將可變對象作為參數并不一定使它成為過程
這是一個函數接受一個可變對象（不是過程）

```python
peek_str s: Str! = log s

make_proc(x!: (Int => Int)): (Int => Int) = y => x! y
p! = make_proc(x => x)
print! p! 1 # 1
```

此外，過程和函數通過`proc :> func`關聯
因此，可以在過程中定義函數
但是，請注意，反過來是不可能的

```python
proc!(x: Int!) = y -> log x, y # OK
func(x: Int) = y => print! x, y # NG
```

## 綁定

過程可以操作范圍外的變量。

```python
x = !0
proc!() =
    x.inc!()
proc!()
assert x == 1
```

此時，“proc！” 具有以下類型：

```python
proc!: {| x: Int! |} () => ()
```

`{| x: Int! |}`部分稱為綁定列，表示過程操作的變量及其類型。
綁定列是自動派生的，因此無需顯式編寫。
請注意，常規過程只能操作預先確定的外部變量。 這意味著不能重寫傳遞給參數的變量。
如果要執行此操作，則必須使用過程方法。 過程方法可以重寫“自”。

```python
C! N = Class {arr = [Int; N]!}
C!.
    new() = Self!(0) {arr = ![]}
C!(N).
    # push!: {|self: C!(N) ~> C!(N+1)|} (self: RefMut(C!(N)), x: Int) => NoneType
    push! ref! self, x = self.arr.push!(x)
    # pop!: {|self: C!(N) ~> C!(N-1)|} (self: RefMut(C!(N))) => Int
    pop! ref! self = self.arr.pop!()
c = C!.new()
c.push!(1)
assert c.pop!() ==  1
```

<p align='center'>
    <a href='./07_side_effect.md'>上一頁</a> | <a href='./09_builtin_procs.md'>下一頁</a>
</p>
