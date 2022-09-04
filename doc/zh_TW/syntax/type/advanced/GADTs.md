# Generalized Algebraic Data Types，GADTs

Erg 可以通過對 Or 類型進行類化來創建廣義代數數據類型（GADTs）。


```erg
Nil T = Class(Impl := Phantom T)
Cons T = Class {head = T; rest = List T}, Impl := Unpack
List T: Type = Class(Nil T or Cons T)
List.
    nil|T|() = Self(T).new Nil(T).new()
    cons head, rest | T = Self(T).new Cons(T).new(head, rest)
    head self = match self:
        {head; ...}: Cons _ -> head
        _: Nil -> panic "empty list"
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil.head() # RuntimeError: "empty list"
```

將作為<gtr=“5”/>而不是<gtr=“5”/>是因為使用時不需要類型。


```erg
i = List.nil()
_: List Int = cons 1, i
```

這裡定義的是 GADTs，但它是一個簡單的實現，沒有真正的 GADTs 價值。例如，如果內容為空，上面的<gtr=“8”/>方法將導致運行時錯誤，但可以在編譯時執行此檢查。


```erg
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
print! nil().head() # TypeError
```

在街頭巷尾經常被說明的 GADTs 的例子，是像以上那樣根據類型能判定內容是否為空的列表。 Erg 提供了更精確的定義長度列表的方法。


```erg
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
print! cons(1, nil).pair() # TypeError
print! cons(1, nil).head() # 1
print! nil.head() # TypeError
```