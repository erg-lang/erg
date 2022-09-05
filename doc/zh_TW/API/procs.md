# 过程

## print!

```python
打印！(x)->无类型
```

   使用换行符返回 x。

## 调试&排除;

```python
调试！(x，类型=信息)-> NoneType
```

用换行符调试 x(文件名、行号、变量名一起显示)。 在发布模式中删除。
支持表情符号的终端根据类型加前缀。

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

## for!i: Iterable T, block: T => NoneType

以块的动作遍历迭代器。

## while!cond: Bool!, block: () => NoneType

当cond为True时的执行块。

## Lineno!() -> Nat

## Filename!() -> Str

## Namespace!() -> Str

## Module!() -> Module