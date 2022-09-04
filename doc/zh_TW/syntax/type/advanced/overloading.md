# 過載

Erg 不支持。也就是說，不能對函數卡印進行多重定義（過載）。但是，通過組合trait類和補丁，可以再現過載的行為。可以使用trait而不是trait類，但在這種情況下，安裝<gtr=“8”/>的所有類型都成為對象。


```erg
Add1 = Trait {
    .add1: Self.() -> Self
}
IntAdd1 = Patch Int, Impl := Add1
IntAdd1.
    add1 self = self + 1
RatioAdd1 = Patch Ratio, Impl := Add1
RatioAdd1.
    add1 self = self + 1.0

add1|X <: Add1| x: X = x.add1()
assert add1(1) == 2
assert add1(1.0) == 2.0
```

這種通過接受某一類型的所有亞型而產生的多相稱為。 Erg 中的亞分型多相也包括列多相。

如果各型的處理完全相同，也可以寫如下。上面的寫法用於不同類的行為（但返回類型相同）。使用類型參數的多相稱為。參數多相與如下所示的部分型指定並用的情況較多，這種情況下是參數多相和子分型多相的組合技術。


```erg
add1|T <: Int or Str| x: T = x + 1
assert add1(1) == 2
assert add1(1.0) == 2.0
```

另外，自變量數不同類型的過載可以用默認自變量再現。


```erg
C = Class {.x = Int; .y = Int}
C.
    new(x, y := 0) = Self::__new__ {.x; .y}

assert C.new(0, 0) == C.new(0)
```

雖然無法定義根據自變量的數量類型不同等行為完全變化的函數，但 Erg 採取的立場是，如果行為本來就不同，就應該賦予其他名稱。

結論是，Erg 禁止過載而採用亞分 + 參數多相是出於以下原因。

首先，被超載的函數的定義是分散的。因此，發生錯誤時很難報告原因所在。另外，通過導入子程序，可能會改變已經定義的子程序的行為。


```erg
{id; ...} = import "foo"
...
id x: Int = x
...
id x: Ratio = x
...
id "str" # TypeError: id is not implemented for Str
# But... where did this error come from?
```

其次，與默認參數不匹配。當有默認參數的函數被重載時，存在哪個優先的問題。


```erg
f x: Int = ...
f(x: Int, y := 0) = ...

f(1) # which is chosen?
```

再者，與宣言不相匹配。聲明無法確定指的是哪一個定義。因為<gtr=“13”/>和<gtr=“14”/>沒有包含關係。


```erg
f: Num -> Num
f(x: Int): Ratio = ...
f(x: Ratio): Int = ...
```

而且，破壞語法的連貫性。雖然 Erg 禁止變量的再代入，但是過載的語法看起來像是再代入。也不能替換為無名函數。


```erg
# same as `f = x -> body`
f x = body

# same as... what?
f x: Int = x
f x: Ratio = x
```