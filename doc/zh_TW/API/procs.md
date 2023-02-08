# 過程

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/procs.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/procs.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## print!

```python
打印！(x)->無類型
```

   使用換行符返回 x

## 調試&排除;

```python
調試！(x，類型=信息)-> NoneType
```

用換行符調試 x(文件名、行號、變量名一起顯示)。在發布模式中刪除
支持表情符號的終端根據類型加前綴

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

## for!i: Iterable T, block: T => NoneType

以塊的動作遍歷迭代器

## while! cond!: () => Bool, block!: () => NoneType

當cond!()為True時的執行塊

## Lineno!() -> Nat

## Filename!() -> Str

## Namespace!() -> Str

## Module!() -> Module
