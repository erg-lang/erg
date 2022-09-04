# プロシージャ

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

## while! cond: Bool!, block: () => NoneType

condがTrueの間、blockを実行する。

## Lineno!() -> Nat

## Filename!() -> Str

## Namespace!() -> Str

## Module!() -> Module
