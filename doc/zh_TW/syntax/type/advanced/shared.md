# 共享引用（Shared Reference）

共享引用是一種必須小心處理的語言功能。例如，在 TypeScript 中，以下代碼通過類型檢查。


```typescript
class NormalMember {}
class VIPMember extends NormalMember {}

let vip_area: VIPMember[] = []
let normal_area: NormalMember[] = vip_area

normal_area.push(new NormalMember())
console.log(vip_area) # [NormalMember]
```

普通會員闖入了貴賓區。這是一個明顯的 bug，有什麼不對呢？原因是共享引用的。 <gtr=“6”/>是通過複製<gtr=“7”/>而創建的，但其類型已發生變化。但是，由於<gtr=“8”/>繼承了<gtr=“9”/>，所以<gtr=“10”/>被認為是沒有問題的。 <gtr=“11”/>關係對於不變對象來說是沒有問題的。但是，如果像上面那樣進行破壞性操作，就會出現破綻。

在 Erg 中，由於所有權系統，這些代碼被彈出。


```erg
NormalMember = Class()
VIPMember = Class()

vip_area = [].into [VIPMember; !_]
normal_area: [NormalMember; !_] = vip_area

normal_area.push!(NormalMember.new())
log vip_area # OwnershipError: `vip_room` was moved to `normal_room`
```

但是，在某些情況下，只有一個對象的所有權是不方便的。為此，Erg 的類型為，它表示共享狀態。


```erg
$p1 = SharedCell!.new(!1)
$p2 = $p1.mirror!()
$p3 = SharedCell!.new(!1)
# $p1 == $p2 比較內容類型 Int!
assert $p1 == $p2
assert $p1 == $p3
# 檢查 `.addr!` 以查看 $p1 和 $p2 是否相同
assert $p1.addr!() == $p2.addr!()
assert $p1.addr!() != $p3.addr!()
$p1.add! 1
assert $p1 == 2
assert $p2 == 2
assert $p3 == 1
```

類型的對象必須以<gtr=“15”/>開頭。此外，由於其性質，它不能是常數。

類型也是<gtr=“17”/>類型的子類型，可以調用<gtr=“18”/>類型的方法。類型特定的方法只有<gtr=“19”/>、<gtr=“20”/>、<gtr=“21”/>和<gtr=“22”/>。

一個重要的事實是，是非變態的。即，不定義不同類型參數的包含關係。


```erg
$vip_area = SharedCell!.new([].into [VIPMember; !_])
$normal_area: SharedCell!([NormalMember; !_]) = $vip_area.mirror!() # TypeError: expected SharedCell!([NormalMember; !_]), but got SharedCell!([VIPMember; !_])
# hint: SharedCell!(T) is non-variant, which means it cannot have a supertype or a subtype.
```

但是下面的代碼沒有問題。在最後一行中，類型轉換為參數。


```erg
$normal_area = SharedCell!.new([].into [NormalMember; !_])
$normal_area.push!(NormalMember.new()) # OK
$normal_area.push!(VIPMember.new()) # OK
```