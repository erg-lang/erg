# Shared Reference

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/shared.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/shared.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

Shared references are one of those language features that must be handled with care.
In TypeScript, for example, the following code will pass type checking.

```typescript
class NormalMember {}
class VIPMember extends NormalMember {}

let vip_area: VIPMember[] = []
let normal_area: NormalMember[] = vip_area

normal_area.push(new NormalMember())
console.log(vip_area) # [NormalMember]
```

A NormalMember has entered the vip_area. It is an obvious bug, however what went wrong?
The cause is the shared reference [denatured](./variance.md). The `normal_area` is created by copying the `vip_area`, but in doing so the type has changed.
But `VIPMember` inherits from `NormalMember`, so `VIPMember[] <: NormalMember[]`, and this is not a problem.
The relation `VIPMember[] <: NormalMember[]` is fine for immutable objects. However, if you perform a destructive operation like the one above, there will be a breakdown.

In Erg, such code is played back due to the ownership system.

```erg
NormalMember = Class()
VIPMember = Class()

vip_area = [].into [VIPMember; !_]
normal_area: [NormalMember; !_] = vip_area

normal_area.push!(NormalMember.new())
log vip_area # OwnershipError: `vip_room` was moved to `normal_room`
```

However, it can be inconvenient for an object to be owned by only one place.
For this reason, Erg has a type `SharedCell!T!`, which represents a shared state.

```erg
$p1 = SharedCell!.new(!1)
$p2 = $p1.mirror!()
$p3 = SharedCell!.new(!1)
# If $p1 == $p2, a comparison of the content type Int!
assert $p1 == $p2
assert $p1 == $p3
# Check if $p1 and $p2 point to the same thing with `.addr!`.
assert $p1.addr!() == $p2.addr!()
assert $p1.addr!() != $p3.addr!()
$p1.add! 1
assert $p1 == 2
assert $p2 == 2
assert $p3 == 1
```

Objects of type `SharedCell!` must be prefixed with `$`. Also, by their nature, they cannot be constants.

The `SharedCell! T!` type is also a subtype of `T!` and can call methods of type `T!`. The only methods specific to the `SharedCell!T!` type are `.addr!`, `.mirror!` and `.try_take`.

An important fact is that `SharedCell! T!` is non-variant, i.e., no inclusions are defined for different type arguments.

```erg
$vip_area = SharedCell!.new([].into [VIPMember; !_])
$normal_area: SharedCell!([NormalMember; !_]) = $vip_area.mirror!() # TypeError: expected SharedCell!([NormalMember; !_]), but got SharedCell!([VIPMember; !_])
# hint: SharedCell!(T) is non-variant, which means it cannot have a supertype or a subtype.
```

However, the following code have not problem. In the last line, it's the `VIPMember` argument that has been typed converted.

```erg
$normal_area = SharedCell!.new([].into [NormalMember; !_])
$normal_area.push!(NormalMember.new()) # OK
$normal_area.push!(VIPMember.new()) # OK
```
