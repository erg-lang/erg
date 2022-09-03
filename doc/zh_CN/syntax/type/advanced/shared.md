# 共享引用（Shared Reference）

共享引用是一种必须小心处理的语言功能。例如，在 TypeScript 中，以下代码通过类型检查。


```typescript
class NormalMember {}
class VIPMember extends NormalMember {}

let vip_area: VIPMember[] = []
let normal_area: NormalMember[] = vip_area

normal_area.push(new NormalMember())
console.log(vip_area) # [NormalMember]
```

普通会员闯入了贵宾区。这是一个明显的 bug，有什么不对呢？原因是共享引用的。<gtr=“6”/>是通过复制<gtr=“7”/>而创建的，但其类型已发生变化。但是，由于<gtr=“8”/>继承了<gtr=“9”/>，所以<gtr=“10”/>被认为是没有问题的。<gtr=“11”/>关系对于不变对象来说是没有问题的。但是，如果像上面那样进行破坏性操作，就会出现破绽。

在 Erg 中，由于所有权系统，这些代码被弹出。


```erg
NormalMember = Class()
VIPMember = Class()

vip_area = [].into [VIPMember; !_]
normal_area: [NormalMember; !_] = vip_area

normal_area.push!(NormalMember.new())
log vip_area # OwnershipError: `vip_room` was moved to `normal_room`
```

但是，在某些情况下，只有一个对象的所有权是不方便的。为此，Erg 的类型为，它表示共享状态。


```erg
$p1 = SharedCell!.new(!1)
$p2 = $p1.mirror!()
$p3 = SharedCell!.new(!1)
# $p1 == $p2とすると、中身の型Int!の比較が行われる
assert $p1 == $p2
assert $p1 == $p3
# $p1と$p2が同じものを指しているかは、`.addr!`で確認する
assert $p1.addr!() == $p2.addr!()
assert $p1.addr!() != $p3.addr!()
$p1.add! 1
assert $p1 == 2
assert $p2 == 2
assert $p3 == 1
```

类型的对象必须以<gtr=“15”/>开头。此外，由于其性质，它不能是常数。

类型也是<gtr=“17”/>类型的子类型，可以调用<gtr=“18”/>类型的方法。类型特定的方法只有<gtr=“19”/>、<gtr=“20”/>、<gtr=“21”/>和<gtr=“22”/>。

一个重要的事实是，是非变态的。即，不定义不同类型参数的包含关系。


```erg
$vip_area = SharedCell!.new([].into [VIPMember; !_])
$normal_area: SharedCell!([NormalMember; !_]) = $vip_area.mirror!() # TypeError: expected SharedCell!([NormalMember; !_]), but got SharedCell!([VIPMember; !_])
# hint: SharedCell!(T) is non-variant, which means it cannot have a supertype or a subtype.
```

但是下面的代码没有问题。在最后一行中，类型转换为参数。


```erg
$normal_area = SharedCell!.new([].into [NormalMember; !_])
$normal_area.push!(NormalMember.new()) # OK
$normal_area.push!(VIPMember.new()) # OK
```
