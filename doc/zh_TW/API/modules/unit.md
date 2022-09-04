# module `unit`

`unit`模塊是將數值計算中常用的單位定義為型的模塊。
Erg的數值型有`Nat`， `Int`， `Ratio`等。但是，這些模型並沒有掌握“這意味著什麼”的信息，只能進行米和碼之間的加法等荒唐的計算。
通過使用`unit`模塊，可以防止將單位不同的數值傳遞給函數的錯誤。
這樣的失誤是實際發生,而且[單位系的錯誤火星探測器失踪](http://www.sydrose.com/case100/287/)等,可能引起嚴重的bug。
如果想在進行數值計算的基礎上提高代碼的魯棒性的話應該事先使用這個模塊。

``` erg
{*} = import "unit"

等價於x = 6m# `x = Meter.new(6)`
等價於t = 3s# `t = c.new(3)`
# m/s是速度的單位對象，Velocity型
print !x/ t# 2m/s
print !x + 4m# 10m
print !x + 2s# TypeError: `+`(Meter, Sec) is not implemented
```

`m`， `s`， `m/s`這樣的對像被稱為單位對象。它本身就有1m, 1s, 1m/s的意思。 `m/s`可以說是m和s的合成產生的單位對象。

unit將以下單位定義為型。國際單位制(SI)。

* 長度:Meter(單位常數:m)
* 質量:KiloGram(單位常數:kg, g = 0.001kg)
* 時間:Sec(分鐘、時間、日、年等有由Sec生成的minute, hour, day, year等常數)
* 電流:Amper(單位常數:a)
* 溫度:Kelvin(單位常數:k, Fahren，也有Celsius型，可相互轉換)
* 物質的量:Mol(單位常數:Mol)
* 光度:Candela(單位常數:cd)

另外，`Unit1`， `UnitMul`， `UnitDiv`這樣的類型被定義，使用這個可以合成基本類型創建新的單位。
例如，振動頻率的單位赫茲(hertz)是用振動週期(秒)的倒數來定義的，所以`UnitDiv(Unit1, Sec)`。
想要把這個類型視為有意義的類型(想要加上專用的方法，等等)的時候，[補丁](./../ . ./syntax/type/07_patch.md)。

``` erg
Hertz = Patch UnitDiv(Unit1, Sec)
SquareMeter = UnitMul(Meter, Meter)
```

輔助單位也被預先定義了幾個。

* 振動頻率:Hertz(hz)
* 力:Newton。
* 能量:Joule(j)
* 工作量:Watt(w)
* 電勢:Volt(v)
* 電阻:Ohm(Ohm)
* 速度:Velocity(m/s)
* 面積:SquareMeter(m**2)
* 體積:CubicMeter(m**3) (litre = 10e- 3m **3)
* 角度:Degree(deg) (rad = 180/pi deg)
* 長度:Feet, Yard, Inch, Mile, Ly, Au, Angstrom
* 重量:Pond

另外，前綴也有定義。

* Femto = 1e-15
* Pico = 1e-12
* Nano = 1e-9
* Micro = 1e-6
* Milli = 1e-3
* Centi = 1e-2
* Deci = 1e-1
* Hecto = 1e+2
* Kilo = 1e+3
* Mega = 1e+6
* Giga = 1e+9
* Tera = 1e+12
* Peta = 1e+15
* Exa = 1e+18

※與名字的由來相反，Erg基本上採用MKS單位制。 cgs單位系unit模塊的希望時,外部庫([cgs] (https://github.com/mtshiba/cgs)等),請使用。