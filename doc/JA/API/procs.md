# プロシージャ

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/procs.md%26commit_hash%3D9b1457b695da9dc0f071091ded48f068ed545083)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/procs.md&commit_hash=9b1457b695da9dc0f071091ded48f068ed545083)

## print!

```python
print!(x) -> NoneType
```

  xを改行ありで返す。

## debug&excl;

```python
debug!(x, type = Info) -> NoneType
```

xを改行ありでデバッグ表示(ファイル名、行数、変数の場合変数名が一緒に表示される)する。リリースモードでは除去される。
絵文字対応ターミナルではtypeに応じてプレフィックスが付く。

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

## for! i: Iterable T, block: T => NoneType

blockの動作でイテレータを走査する。

## while! cond!: () => Bool, block!: () => NoneType

cond!()がTrueの間、block!を実行する。

## Lineno!() -> Nat

## Filename!() -> Str

## Namespace!() -> Str

## Module!() -> Module
