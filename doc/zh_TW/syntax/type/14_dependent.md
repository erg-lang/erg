# 依賴類型

依賴類型是一個特性，可以說是 Erg 的最大特性。
依賴類型是將值作為參數的類型。 普通的多態類型只能將類型作為參數，但依賴類型放寬了這個限制。

依賴類型等價于`[T; N]`(`數組(T，N)`)。
這種類型不僅取決于內容類型“T”，還取決于內容數量“N”。 `N` 包含一個`Nat` 類型的對象。

```python
a1 = [1, 2, 3]
assert a1 in [Nat; 3]
a2 = [4, 5, 6, 7]
assert a1 in [Nat; 4]
assert a1 + a2 in [Nat; 7]
```

如果函數參數中傳遞的類型對象與返回類型有關，則寫：

```python
narray: |N: Nat| {N} -> [{N}; N]
narray(N: Nat): [N; N] = [N; N]
assert array(3) == [3, 3, 3]
```

定義依賴類型時，所有類型參數都必須是常量。

依賴類型本身存在于現有語言中，但 Erg 具有在依賴類型上定義過程方法的特性

```python
x=1
f x =
    print! f::x, module::x

# Phantom 類型有一個名為 Phantom 的屬性，其值與類型參數相同
T X: Int = Class Impl := Phantom X
T(X).
    x self = self::Phantom

T(1).x() # 1
```

可變依賴類型的類型參數可以通過方法應用程序進行轉換。
轉換規范是用 `~>` 完成的

```python
# 注意 `Id` 是不可變類型，不能轉換
VM!(State: {"stopped", "running"}! := _, Id: Nat := _) = Class(..., Impl := Phantom! State)
VM!().
    # 不改變的變量可以通過傳遞`_`省略。
    start! ref! self("stopped" ~> "running") =
        self.initialize_something!()
        self::set_phantom!("running")

# 你也可以按類型參數切出(僅在定義它的模塊中)
VM!.new() = VM!(!"stopped", 1).new()
VM!("running" ~> "running").stop!ref!self =
    self.close_something!()
    self::set_phantom!("stopped")

vm = VM!.new()
vm.start!()
vm.stop!()
vm.stop!() # 類型錯誤：VM!(!"stopped", 1) 沒有 .stop!()
# 提示：VM!(!"running", 1) 有 .stop!()
```

您還可以嵌入或繼承現有類型以創建依賴類型。

```python
MyArray(T, N) = Inherit[T; N]

# self 的類型：Self(T, N) 與 .array 一起變化
MyStruct!(T, N: Nat!) = Class {.array: [T; !N]}
```