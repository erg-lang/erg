# 广义代数数据类型 (GADT)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/GADTs.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/GADTs.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Erg 可以通过对 Or 类型进行分类来创建广义代数数据类型 (GADT)。

```python
Nil T = Class(Impl := Phantom T)
Cons T = Class {head = T; rest = List T}, Impl := Unpack
List T: Type = Class(Nil T or Cons T)
List.
    nil|T|() = Self(T).new Nil(T).new()
    cons head, rest | T = Self(T).new Cons(T).new(head, rest)
    head self = match self:
        {head; ...}: Cons_ -> head
        _: Nil -> panic "empty list"
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil.head() # 运行时错误：“空list”
```

我们说 `List.nil|T|() = ...` 而不是 `List(T).nil() = ...` 的原因是我们在使用它时不需要指定类型。

```python
i = List.nil()
_: List Int = cons 1, i
```

这里定义的 `List T` 是 GADTs，但它是一个幼稚的实现，并没有显示 GADTs 的真正价值。
例如，上面的 .head 方法会在 body 为空时抛出运行时错误，但是这个检查可以在编译时进行。

```python
List: (Type, {"Empty", "Nonempty"}) -> Type
List T, "Empty" = Class(Impl := Phantom T)
List T, "Nonempty" = Class {head = T; rest = List(T, _)}, Impl := Unpack
List.
    nil|T|() = Self(T, "Empty").new Nil(T).new()
    cons head, rest | T = Self(T, "Nonempty").new {head; rest}
List(T, "Nonempty").
    head {head; ...} = head
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil().head() # 类型错误
```

街上经常解释的 GADT 的一个例子是一个列表，可以像上面那样通过类型来判断内容是否为空。
Erg 可以进一步细化以定义一个有长度的列表。

```python
List: (Type, Nat) -> Type
List T, 0 = Class(Impl := Phantom T)
List T, N = Class {head = T; rest = List(T, N-1)}, Impl := Unpack
List.
    nil|T|() = Self(T, 0).new Nil(T).new()
    cons head, rest | T, N = Self(T, N).new {head; rest}
List(_, N | N >= 1).
    head {head; ...} = head
List(_, N | N >= 2).
    pair {head = first; rest = {head = second; ...}} = [first, second]
{nil; cons; ...} = List

print! cons(1, cons(2, nil)).pair() # [1, 2]
print! cons(1, nil).pair() # 类型错误
print! cons(1, nil).head() # 1
print! nil. head() # 类型错误
```