# 過程

## print!

```python
打印！(x)->無類型
```

   使用換行符返回 x。

## 調試&排除;

```python
調試！(x，類型=信息)-> NoneType
```

用換行符調試 x(文件名、行號、變量名一起顯示)。 在發布模式中刪除。
支持表情符號的終端根據類型加前綴。

* type == Info: ??
* type == Ok: ?
* type == Warn: ??
* type == Hint: ??

## for!i: Iterable T, block: T => NoneType

以塊的動作遍歷迭代器。

## while!cond: Bool!, block: () => NoneType

當cond為True時的執行塊。

## Lineno!() -> Nat

## Filename!() -> Str

## Namespace!() -> Str

## Module!() -> Module