# 模块`unit`

`unit` 模块是将数值计算中经常使用的单位定义为类型的模块。
Erg 数值类型包括 `Nat`、`Int`、`Ratio` 等。但是，这些类型没有关于“数字的含义”的信息，因此可以执行诸如添加米和码之类的无意义计算。
通过使用 `unit` 模块，您可以避免错误，例如将不同单位的数字传递给函数。
这样的错误确实会发生，并且会导致诸如[由于错误的单位系统导致火星探测器丢失](http://www.sydrose.com/case100/287/)之类的严重错误。
如果您希望代码在进行数值计算时更加健壮，您应该使用此模块。

```python
{*} = import "unit"

x = 6m # 相当于 `x = Meter.new(6)`
t = 3s # 相当于 `t = Sec.new(3)`
#m/s是速度单位对象，类型为velocity
print! x/t # 2m/s
print! x + 4m # 10m
print! x + 2s # 类型错误: `+`(Meter, Sec) 未实现
```
对象`m`、`s`和`m/s`被称为单元对象。它本身具有1m、1s、1m/s的含义。 `m/s`可以说是结合m和s创建的单位对象。

在单元中，以下单元被定义为类型。它被称为SI(国际单位制)。

* 长度：Meter(单位常数：m)
* 质量：KiloGram(单位常数：kg，g = 0.001kg)
* 时间：Sec(分、时、日、年等有分、时、日、年等常量由秒生成)
* 电流：Amper(单位常数：a)
* 温度：Kelvin(单位常数：k。华氏、摄氏度类型也可用，可相互转换)
* 物质量：Mol(单位常数：mol)
* 发光强度：Candela(单位常数：cd)

此外，还定义了`Unit1`、`UnitMul`和`UnitDiv`类型，可以通过组合基本类型来创建新的单元。
例如`UnitDiv(Unit1, Sec)`，因为频率单位赫兹(hertz)被定义为振动周期(秒)的倒数。
如果要将此类型视为有意义的类型(例如添加专用方法)，则应创建 [patch](./../../syntax/type/07_patch.md)。

```python
Hertz = Patch UnitDiv(Unit1, Sec)
SquareMeter = Patch UnitMul(Meter, Meter)
```

一些辅助单元也是预定义的:

* 频率: Hertz(hz)
* 力:   Newton(newton)
* 能量: Joule(j)
* 功率: Watt(w)
* 电压: Volt(v)
* 电阻: Ohm(ohm)
* 速度: Velocity(m/s)
* 面积: SquareMeter(m^2)
* 体积: CubicMeter(m^3) (liter = 10e-3 m^3)
* 角度: Degree(deg) (rad = 180/pi deg)
* 长度: Feet, Yard, Inch, Mile, Ly, Au, Angstrom
* 重量: Pound

它还定义了一个前缀:

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

* 与名字的由来相反，Erg基本采用MKS单位制。如果需要 CGS 单位制的单位模块，请使用外部库[cgs](https://github.com/mtshiba/cgs)等)。