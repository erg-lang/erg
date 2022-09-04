# Algebraic type（代數類型）

代數運算類型是指將類型視為代數並進行運算而產生的類型。代數類型處理的操作包括 Union、Intersection、Diff 和 Complement。常規類只能是 Union，其他操作都是類型錯誤。

## Union

在 Union 型中，關於型可以給出多個可能性。顧名思義，它由運算符生成。典型的 Union 是<gtr=“8”/>類型。 <gtr=“9”/>類型是<gtr=“10”/>的 patch type，主要表示可能失敗的值。


```erg
IntOrStr = Int or Str
assert dict.get("some key") in (Int or NoneType)

# 暗黙に`T != NoneType`となる
Option T = T or NoneType
```

## Intersection

Intersection 類型是通過操作組合類型而得到的。


```erg
Num = Add and Sub and Mul and Eq
```

如上所述，無法通過操作將常規類組合在一起。實例屬於唯一的類。

## Diff

Diff 類型是通過運算得到的。作為接近英文的表記，<gtr=“14”/>比較好，但由於<gtr=“15”/>，<gtr=“16”/>並列比較好，所以建議只使用<gtr=“17”/>。


```erg
CompleteNum = Add and Sub and Mul and Div and Eq and Ord
Num = CompleteNum not Div not Ord

True = Bool not {False}
OneTwoThree = {1, 2, 3, 4, 5, 6} - {4, 5, 6, 7, 8, 9, 10}
```

## Complement

Complement 是通過運算得到的，但它是一元運算。 <gtr=“19”/>類型是<gtr=“20”/>的縮寫符號。 <gtr=“21”/>類型的 Intersection 等效於 Diff，<gtr=“22”/>類型的 Diff 等效於 Intersection。但不建議這樣的寫法。


```erg
# the simplest definition of the non-zero number type
NonZero = Not {0}
# deprecated styles
{True} == Bool and not {False} # 1 == 2 + - 1
Bool == {True} not not {False} # 2 == 1 - -1
```

## 真實代數類型

代數運算類型包括可簡化的表觀代數運算類型和不能進一步簡化的“真代數運算類型”。其他“表觀代數類型”包括 Enum 和 Interval 類型，以及和<gtr=“24”/>記錄類型。因為它們可以簡化，所以它們不是真正的代數運算類型，如果用來指定類型，就會出現 Warning。要消除警告，必須簡化或定義類型。


```erg
assert {1, 2, 3} or {2, 3} == {1, 2, 3}
assert {1, 2, 3} and {2, 3} == {2, 3}
assert -2..-1 or 1..2 == {-2, -1, 1, 2}

i: {1, 2} or {3, 4} = 1 # TypeWarning: {1, 2} or {3, 4} can be simplified to {1, 2, 3, 4}
p: {x = Int, ...} and {y = Int; ...} = {x = 1; y = 2; z = 3}
# TypeWaring: {x = Int, ...} and {y = Int; ...} can be simplified to {x = Int; y = Int; ...}

Point1D = {x = Int; ...}
Point2D = Point1D and {y = Int; ...} # == {x = Int; y = Int; ...}
q: Point2D = {x = 1; y = 2; z = 3}
```

真正的代數類型包括類型和<gtr=“26”/>類型。類之間的<gtr=“27”/>等類型為<gtr=“28”/>類型。


```erg
assert Int or Str == Or(Int, Str)
assert Int and Marker == And(Int, Marker)
```

Diff 和 Complement 類型不是真正的代數運算類型，因為它們總是可以簡化。