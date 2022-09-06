# 廣義代數數據類型 (GADT)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/GADTs.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/GADTs.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Erg 可以通過對 Or 類型進行分類來創建廣義代數數據類型 (GADT)。

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
print! nil.head() # 運行時錯誤：“空list”
```

我們說 `List.nil|T|() = ...` 而不是 `List(T).nil() = ...` 的原因是我們在使用它時不需要指定類型。

```python
i = List.nil()
_: List Int = cons 1, i
```

這里定義的 `List T` 是 GADTs，但它是一個幼稚的實現，并沒有顯示 GADTs 的真正價值。
例如，上面的 .head 方法會在 body 為空時拋出運行時錯誤，但是這個檢查可以在編譯時進行。

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
print! nil().head() # 類型錯誤
```

街上經常解釋的 GADT 的一個例子是一個列表，可以像上面那樣通過類型來判斷內容是否為空。
Erg 可以進一步細化以定義一個有長度的列表。

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
print! cons(1, nil).pair() # 類型錯誤
print! cons(1, nil).head() # 1
print! nil. head() # 類型錯誤
```