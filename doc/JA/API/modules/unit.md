# module `unit`

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/modules/unit.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/modules/unit.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

`unit`モジュールは数値計算でよく使われる単位を型として定義したモジュールです。
Ergの数値型は`Nat`, `Int`, `Ratio`などがあります。しかしこれらの型は「何を意味する数値なのか」という情報を持っておらず、メートルとヤード同士の足し算などといったナンセンスな計算を行えてしまいます。
`unit`モジュールを使うことにより、単位の違う数値を関数に渡すといったミスを防げます。
このようなミスは実際に起っており、[単位系の取り間違いで火星探査機が行方不明](http://www.sydrose.com/case100/287/)になるなど、深刻なバグを引き起こしかねません。
数値計算を行う上でコードの堅牢性を高めたいならばこのモジュールを使用しておくべきです。

```python
{*} = import "unit"

x = 6m # `x = Meter.new(6)`と等価
t = 3s # `t = Sec.new(3)`と等価
# m/sは速度の単位オブジェクトで、Velocity型
print! x/t # 2m/s
print! x + 4m # 10m
print! x + 2s # TypeError: `+`(Meter, Sec) is not implemented
```

`m`, `s`, `m/s`というオブジェクトは単位オブジェクトと呼ばれます。それ自体で1m, 1s, 1m/sという意味を持ちます。`m/s`はmとsの合成によって生まれた単位オブジェクトといえます。

unitでは以下の単位を型として定義しています。SI(国際単位系)と呼ばれるものです。

* 長さ：Meter(単位定数: m)
* 質量：KiloGram(単位定数: kg, g = 0.001kg)
* 時間：Sec (分、時間、日、年などはSecから生成されたminute, hour, day, yearなどの定数がある)
* 電流: Amper(単位定数: a)
* 温度: Kelvin(単位定数: k, Fahren, Celsius型もあり、相互変換可能)
* 物質量: Mol(単位定数: mol)
* 光度: Candela(単位定数: cd)

また、`Unit1`, `UnitMul`, `UnitDiv`という型が定義されており、これを使用して基本型を合成し新しい単位を作成する事が可能です。
例えば、振動数の単位ヘルツ(hertz)は振動周期(秒)の逆数で定義されているので、`UnitDiv(Unit1, Sec)`です。
この型を意味のある型とみなしたい(専用のメソッドを加えたい、など)ときは、[パッチ](./../../syntax/type/07_patch.md)を作成すると良いでしょう。

```python
Hertz = Patch UnitDiv(Unit1, Sec)
SquareMeter = Patch UnitMul(Meter, Meter)
```

補助単位も予めいくつか定義されています。

* 振動数: Hertz(hz)
* 力: Newton(newton)
* エネルギー: Joule(j)
* 仕事率: Watt(w)
* 電位: Volt(v)
* 電気抵抗: Ohm(ohm)
* 速度: Velocity(m/s)
* 面積: SquareMeter(m^2)
* 体積: CubicMeter(m^3) (litre = 10e-3 m^3)
* 角度: Degree(deg) (rad = 180/pi deg)
* 長さ: Feet, Yard, Inch, Mile, Ly, Au, Angstrom
* 重さ: Pond

また、接頭辞も定義しています。

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

※名前の由来に反して、Ergでは基本的にMKS単位系を採用しています。CGS単位系のunitモジュールが欲しい場合は、外部ライブラリ([cgs](https://github.com/mtshiba/cgs)等)を使用してください。
