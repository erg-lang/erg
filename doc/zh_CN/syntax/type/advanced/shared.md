# 共享参考

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/shared.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/shared.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

共享引用是必须小心处理的语言特性之一
例如，在 TypeScript 中，以下代码将通过类型检查

```typescript
class NormalMember {}
class VIPMember extends NormalMember {}

let vip_area: VIPMember[] = []
let normal_area: NormalMember[] = vip_area

normal_area.push(new NormalMember())
console.log(vip_area) # [NormalMember]
```

一个 NormalMember 已进入 vip_area。 这是一个明显的错误，但是出了什么问题?
原因是共享引用 [denatured](./variance.md)。 `normal_area` 是通过复制 `vip_area` 来创建的，但是这样做的时候类型已经改变了
但是 `VIPMember` 继承自 `NormalMember`，所以 `VIPMember[] <: NormalMember[]`，这不是问题
关系 `VIPMember[] <: NormalMember[]` 适用于不可变对象。 但是，如果您执行上述破坏性操作，则会出现故障

在 Erg 中，由于所有权系统，此类代码会被回放

```python
NormalMember = Class()
VIPMember = Class()

vip_area = [].into [VIPMember; !_]
normal_area: [NormalMember; !_] = vip_area

normal_area.push!(NormalMember.new())
log vip_area # 所有权错误: `vip_room` 已移至 `normal_room`
```

然而，一个对象只属于一个地方可能会很不方便
出于这个原因，Erg 有一个类型 `SharedCell!T!`，它代表一个共享状态

```python
$p1 = SharedCell!.new(!1)
$p2 = $p1.mirror!()
$p3 = SharedCell!.new(!1)
# 如果$p1 == $p2，比较内容类型Int！
assert $p1 == $p2
assert $p1 == $p3
# 检查 $p1 和 $p2 是否用 `.addr!` 指向同一个东西
assert $p1.addr!() == $p2.addr!()
assert $p1.addr!() != $p3.addr!()
$p1.add! 1
assert $p1 == 2
assert $p2 == 2
assert $p3 == 1
```

`SharedCell!` 类型的对象必须以`$` 为前缀。 此外，就其性质而言，它们不可能是常数

`SharedCell！ T!` 类型也是 `T!` 的子类型，可以调用 `T!` 类型的方法。 `SharedCell!T!` 类型特有的唯一方法是 `.addr!`、`.mirror!` 和 `.try_take`

一个重要的事实是`SharedCell! T!` 是非变体的，即没有为不同类型的参数定义包含

```python
$vip_area = SharedCell!.new([].into [VIPMember; !_])
$normal_area: SharedCell!([NormalMember; !_]) = $vip_area.mirror!() #类型错误: 预期 SharedCell！([NormalMember；！_])，但得到 SharedCell！([VIPMember;!_])
# 提示: SharedCell!(T) 是非变体的，这意味着它不能有超类型或子类型
```

但是，下面的代码没有问题。 在最后一行，它是 `VIPMember` 参数已被类型转换

```python
$normal_area = SharedCell!.new([].into [NormalMember; !_])
$normal_area.push!(NormalMember.new()) # OK
$normal_area.push!(VIPMember.new()) # OK
```
