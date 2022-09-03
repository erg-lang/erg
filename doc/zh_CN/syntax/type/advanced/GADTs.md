# Generalized Algebraic Data Types，GADTs

Erg 可以通过对 Or 类型进行类化来创建广义代数数据类型（GADTs）。


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

将作为<gtr=“5”/>而不是<gtr=“5”/>是因为使用时不需要类型。


```erg
i = List.nil()
_: List Int = cons 1, i
```

这里定义的是 GADTs，但它是一个简单的实现，没有真正的 GADTs 价值。例如，如果内容为空，上面的<gtr=“8”/>方法将导致运行时错误，但可以在编译时执行此检查。


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

在街头巷尾经常被说明的 GADTs 的例子，是象以上那样根据类型能判定内容是否为空的列表。Erg 提供了更精确的定义长度列表的方法。


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
