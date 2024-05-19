# 过程

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/procs.md%26commit_hash%3D9b1457b695da9dc0f071091ded48f068ed545083)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/procs.md&commit_hash=9b1457b695da9dc0f071091ded48f068ed545083)

## print!

```python
打印！(x)->无类型
```

   使用换行符返回 x

## 调试&排除;

```python
调试！(x，类型=信息)-> NoneType
```

用换行符调试 x(文件名、行号、变量名一起显示)。在发布模式中删除
支持表情符号的终端根据类型加前缀

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

## for!i: Iterable T, block!: T => NoneType

以块的动作遍历迭代器

## while! cond!: () => Bool, block!: () => NoneType

当cond!()为True时的执行块

## Lineno!() -> Nat

## Filename!() -> Str

## Namespace!() -> Str

## Module!() -> Module
