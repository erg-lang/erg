# module `unit`

`unit`模块是将数值计算中常用的单位定义为型的模块。
Erg的数值型有`Nat`， `Int`， `Ratio`等。但是，这些模型并没有掌握“这意味着什么”的信息，只能进行米和码之间的加法等荒唐的计算。
通过使用`unit`模块，可以防止将单位不同的数值传递给函数的错误。
这样的失误是实际发生,而且[单位系的错误火星探测器失踪](http://www.sydrose.com/case100/287/)等,可能引起严重的bug。
如果想在进行数值计算的基础上提高代码的鲁棒性的话应该事先使用这个模块。

``` erg
{*} = import "unit"

等价于x = 6m# `x = Meter.new(6)`
等价于t = 3s# `t = c.new(3)`
# m/s是速度的单位对象，Velocity型
print !x/ t# 2m/s
print !x + 4m# 10m
print !x + 2s# TypeError: `+`(Meter, Sec) is not implemented
```

`m`， `s`， `m/s`这样的对象被称为单位对象。它本身就有1m, 1s, 1m/s的意思。`m/s`可以说是m和s的合成产生的单位对象。

unit将以下单位定义为型。国际单位制(SI)。

* 长度:Meter(单位常数:m)
* 质量:KiloGram(单位常数:kg, g = 0.001kg)
* 时间:Sec(分钟、时间、日、年等有由Sec生成的minute, hour, day, year等常数)
* 电流:Amper(单位常数:a)
* 温度:Kelvin(单位常数:k, Fahren，也有Celsius型，可相互转换)
* 物质的量:Mol(单位常数:Mol)
* 光度:Candela(单位常数:cd)

另外，`Unit1`， `UnitMul`， `UnitDiv`这样的类型被定义，使用这个可以合成基本类型创建新的单位。
例如，振动频率的单位赫兹(hertz)是用振动周期(秒)的倒数来定义的，所以`UnitDiv(Unit1, Sec)`。
想要把这个类型视为有意义的类型(想要加上专用的方法，等等)的时候，[补丁](./../ . ./syntax/type/07_patch.md)。

``` erg
Hertz = Patch UnitDiv(Unit1, Sec)
SquareMeter = UnitMul(Meter, Meter)
```

辅助单位也被预先定义了几个。

* 振动频率:Hertz(hz)
* 力:Newton。
* 能量:Joule(j)
* 工作量:Watt(w)
* 电势:Volt(v)
* 电阻:Ohm(Ohm)
* 速度:Velocity(m/s)
* 面积:SquareMeter(m**2)
* 体积:CubicMeter(m**3) (litre = 10e- 3m **3)
* 角度:Degree(deg) (rad = 180/pi deg)
* 长度:Feet, Yard, Inch, Mile, Ly, Au, Angstrom
* 重量:Pond

另外，前缀也有定义。

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

※与名字的由来相反，Erg基本上采用MKS单位制。cgs单位系unit模块的希望时,外部库([cgs] (https://github.com/mtshiba/cgs)等),请使用。