# 模塊`unit`

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/modules/unit.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/modules/unit.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

`unit` 模塊是將數值計算中經常使用的單位定義為類型的模塊。
Erg 數值類型包括 `Nat`、`Int`、`Ratio` 等。但是，這些類型沒有關于“數字的含義”的信息，因此可以執行諸如添加米和碼之類的無意義計算。
通過使用 `unit` 模塊，您可以避免錯誤，例如將不同單位的數字傳遞給函數。
這樣的錯誤確實會發生，并且會導致諸如[由于錯誤的單位系統導致火星探測器丟失](http://www.sydrose.com/case100/287/)之類的嚴重錯誤。
如果您希望代碼在進行數值計算時更加健壯，您應該使用此模塊。

```python
{*} = import "unit"

x = 6m # 相當于 `x = Meter.new(6)`
t = 3s # 相當于 `t = Sec.new(3)`
#m/s是速度單位對象，類型為velocity
print! x/t # 2m/s
print! x + 4m # 10m
print! x + 2s # 類型錯誤: `+`(Meter, Sec) 未實現
```
對象`m`、`s`和`m/s`被稱為單元對象。它本身具有1m、1s、1m/s的含義。 `m/s`可以說是結合m和s創建的單位對象。

在單元中，以下單元被定義為類型。它被稱為SI(國際單位制)。

* 長度：Meter(單位常數：m)
* 質量：KiloGram(單位常數：kg，g = 0.001kg)
* 時間：Sec(分、時、日、年等有分、時、日、年等常量由秒生成)
* 電流：Amper(單位常數：a)
* 溫度：Kelvin(單位常數：k。華氏、攝氏度類型也可用，可相互轉換)
* 物質量：Mol(單位常數：mol)
* 發光強度：Candela(單位常數：cd)

此外，還定義了`Unit1`、`UnitMul`和`UnitDiv`類型，可以通過組合基本類型來創建新的單元。
例如`UnitDiv(Unit1, Sec)`，因為頻率單位赫茲(hertz)被定義為振動周期(秒)的倒數。
如果要將此類型視為有意義的類型(例如添加專用方法)，則應創建 [patch](./../../syntax/type/07_patch.md)。

```python
Hertz = Patch UnitDiv(Unit1, Sec)
SquareMeter = Patch UnitMul(Meter, Meter)
```

一些輔助單元也是預定義的:

* 頻率: Hertz(hz)
* 力:   Newton(newton)
* 能量: Joule(j)
* 功率: Watt(w)
* 電壓: Volt(v)
* 電阻: Ohm(ohm)
* 速度: Velocity(m/s)
* 面積: SquareMeter(m^2)
* 體積: CubicMeter(m^3) (liter = 10e-3 m^3)
* 角度: Degree(deg) (rad = 180/pi deg)
* 長度: Feet, Yard, Inch, Mile, Ly, Au, Angstrom
* 重量: Pound

它還定義了一個前綴:

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

* 與名字的由來相反，Erg基本采用MKS單位制。如果需要 CGS 單位制的單位模塊，請使用外部庫[cgs](https://github.com/mtshiba/cgs)等)。