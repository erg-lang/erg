# 共享引用

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/shared.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/shared.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

共享引用是必須小心處理的語言特性之一
例如，在 TypeScript 中，以下代碼將通過類型檢查

```typescript
class NormalMember {}
class VIPMember extends NormalMember {}

let vip_area: VIPMember[] = []
let normal_area: NormalMember[] = vip_area

normal_area.push(new NormalMember())
console.log(vip_area) # [NormalMember]
```

一個 NormalMember 已進入 vip_area。這是一個明顯的錯誤，但是出了什么問題?
原因是共享引用 [denatured](./variance.md)。`normal_area` 是通過復制 `vip_area` 來創建的，但是這樣做的時候類型已經改變了
但是 `VIPMember` 繼承自 `NormalMember`，所以 `VIPMember[] <: NormalMember[]`，這不是問題
關系 `VIPMember[] <: NormalMember[]` 適用于不可變對象。但是，如果您執行上述破壞性操作，則會出現故障

在 Erg 中，由于所有權系統，此類代碼會被回放

```python
NormalMember = Class()
VIPMember = Class()

vip_area = [].into [VIPMember; !_]
normal_area: [NormalMember; !_] = vip_area

normal_area.push!(NormalMember.new())
log vip_area # 所有權錯誤: `vip_room` 已移至 `normal_room`
```

然而，一個對象只屬于一個地方可能會很不方便
出于這個原因，Erg 有一個類型 `SharedCell!T!`，它代表一個共享狀態

```python
$p1 = SharedCell!.new(!1)
$p2 = $p1.mirror!()
$p3 = SharedCell!.new(!1)
# 如果$p1 == $p2，比較內容類型Int！
assert $p1 == $p2
assert $p1 == $p3
# 檢查 $p1 和 $p2 是否用 `.addr!` 指向同一個東西
assert $p1.addr!() == $p2.addr!()
assert $p1.addr!() != $p3.addr!()
$p1.add! 1
assert $p1 == 2
assert $p2 == 2
assert $p3 == 1
```

`SharedCell!` 類型的對象必須以`$` 為前綴。此外，就其性質而言，它們不可能是常數

`SharedCell！ T!` 類型也是 `T!` 的子類型，可以調用 `T!` 類型的方法。`SharedCell!T!` 類型特有的唯一方法是 `.addr!`、`.mirror!` 和 `.try_take`

一個重要的事實是`SharedCell! T!` 是非變體的，即沒有為不同類型的參數定義包含

```python
$vip_area = SharedCell!.new([].into [VIPMember; !_])
$normal_area: SharedCell!([NormalMember; !_]) = $vip_area.mirror!() # 類型錯誤: 預期 SharedCell！([NormalMember；！_])，但得到 SharedCell！([VIPMember;!_])
# 提示: SharedCell!(T) 是非變體的，這意味著它不能有父類型或子類型
```

但是，下面的代碼沒有問題。在最后一行，它是 `VIPMember` 參數已被類型轉換

```python
$normal_area = SharedCell!.new([].into [NormalMember; !_])
$normal_area.push!(NormalMember.new()) # OK
$normal_area.push!(VIPMember.new()) # OK
```
