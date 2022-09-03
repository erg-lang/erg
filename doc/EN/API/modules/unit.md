# module `unit`

The `unit` module is a module that defines units that are often used in numerical calculations as types.
Erg numeric types include `Nat`, `Int`, `Ratio`, and so on. However, these types do not have information about "what the numbers mean", so nonsense calculations such as adding meters and yards can be performed.
By using the `unit` module, you can avoid mistakes such as passing numbers with different units to functions.
Mistakes like this actually occur, and serious bugs such as [Mars probe missing due to wrong unit system](http://www.sydrose.com/case100/287/) can cause it.
You should use this module if you want your code to be more robust when doing numerical computations.

``` erg
{*} = import "unit"

x = 6m # equivalent to `x = Meter.new(6)`
t = 3s # equivalent to `t = Sec.new(3)`
# m/s is a velocity unit object, of type Velocity
print! x/t # 2m/s
print! x + 4m # 10m
print! x + 2s # TypeError: `+`(Meter, Sec) is not implemented
```

The objects `m`, `s`, and `m/s` are called unit objects. It has the meaning of 1m, 1s, 1m/s by itself. `m/s` can be said to be a unit object created by combining m and s.

In unit, the following units are defined as types. It is called SI (International System of Units).

* Length: Meter (unit constant: m)
* Mass: KiloGram (unit constant: kg, g = 0.001kg)
* Time: Sec (minute, hour, day, year, etc. have constants such as minute, hour, day, year generated from Sec)
* Current: Amper (unit constant: a)
* Temperature: Kelvin (unit constant: k, Fahren, Celsius types are also available and can be converted to each other)
* Amount of substance: Mol (unit constant: mol)
* Luminous intensity: Candela (unit constant: cd)

In addition, the types `Unit1`, `UnitMul`, and `UnitDiv` are defined, which can be used to create new units by combining basic types.
For example, `UnitDiv(Unit1, Sec)`, because the unit of frequency hertz (hertz) is defined as the reciprocal of the vibration period (seconds).
If you want to treat this type as a meaningful type (such as adding a dedicated method), you should create a [patch](./../../syntax/type/07_patch.md).

``` erg
Hertz = Patch UnitDiv(Unit1, Sec)
SquareMeter = Patch UnitMul(Meter, Meter)
```

Some auxiliary units are also predefined.

* Frequency: Hertz(hz)
* Force: Newton(newton)
* Energy: Joule(j)
* Power: Watt(w)
* Potential: Volt(v)
* Electrical resistance: Ohm(ohm)
* Velocity: Velocity(m/s)
* Area: SquareMeter(m**2)
* Volume: CubicMeter(m**3) (liter = 10e-3 m**3)
* Angle: Degree(deg) (rad = 180/pi deg)
* Length: Feet, Yard, Inch, Mile, Ly, Au, Angstrom
* Weight: Pound

It also defines a prefix.

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
*Exa = 1e+18

*Contrary to the origin of the name, Erg basically adopts the MKS unit system. If you want the unit module of the CGS unit system, please use an external library ([cgs](https://github.com/mtshiba/cgs) etc.).