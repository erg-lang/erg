# Literal

## 基本的なリテラル

### 整数リテラル(Int Literal)

```python
0, -0, 1, -1, 2, -2, 3, -3, ...
```

### 有理数リテラル(Ratio Literal)

```python
0.00, -0.0, 0.1, 400.104, ...
```

`Ratio`リテラルで整数部分または小数部分が`0`のときは、その`0`を省略できます。

```python
assert 1.0 == 1.
assert 0.5 == .5
```

> __Note__: この`assert`という関数は、`1.0`と`1.`が等しいことを示すために使用しました。
以降のドキュメントでは、結果が等しいことを示すために`assert`を使用する場合があります。

### 文字列リテラル(Str Literal)

Unicodeで表現可能な文字列は、すべて使用できます。
Pythonとは違い、`'`ではクオーテーション(囲み)できません。文字列の中で`"`を使いたいときは`\"`としてください。

```python
"", "a", "abc", "111", "1# 3f2-3*8$", "こんにちは", "السَّلَامُ عَلَيْكُمْ", ...
```

`{}`によって文字列の中に式を埋めこめます。これを文字列補間(string interpolation)といいます。
`{`, `}`自体を出力したい場合は`\{`, `\}`とします。

```python
assert "1 + 1 is 2" == "{1} + {1} is {1+1}"
s = "1+1"
assert "\{1+1}\" == "\{{s}\}"
```

### 指数リテラル(Exponential Literal)

これは学術計算でよく使用される指数表記を表すリテラルです。`Ratio`型のインスタンスになります。
非常に大きな/小さな数を表すときに使用します。Pythonと表記法は同じです。

```python
1e-34, 0.4e-10, 2.455+e5, 245e5, 25E5, ...
```

```python
assert 1e-10 == 0.0000000001
```

## リテラルを組み合わせて生成するもの(複合リテラル)

これらのリテラルは、それぞれ単独で解説されているドキュメントがあるので、詳しくはそちらを参照してください。

### [配列リテラル(Array Literal)](./10_array.md)

```python
[], [1], [1, 2, 3], ["1", "2",], [1, "1", True, [1]], ...
```

### [辞書リテラル(Dict Literal)](./11_dict.md)

```python
{:}, {"one": 1}, {"one": 1, "two": 2}, {"1": 1, "2": 2}, {1: "1", 2: True, "three": [1]}, ...
```

### [組リテラル(Tuple Literal)](./12_tuple.md)

```python
(), (1, 2, 3), (1, "hello", True), ...
```

### [レコードリテラル(Record Literal)](./13_record.md)

```python
{=}, {one = 1}, {one = 1; two = 2}, {.name = "John"; .age = 12}, {.name = Str; .age = Nat}, ...
```

### [集合リテラル(Set Literal)](./14_set.md)

```python
{}, {1}, {1, 2, 3}, {"1", "2", "1"}, {1, "1", True, [1]} ...
```

`Array`リテラルとの違いとして、`Set`では重複する要素が取り除かれます。

```python
assert {1, 2, 1} == {1, 2}
```

### リテラルのように見えるがそうではないもの

## 真偽値オブジェクト(Boolean Object)

```python
True, False
```

### Noneオブジェクト

```python
None
```

## 範囲オブジェクト(Range Object)

```python
assert 0..5 == {1, 2, 3, 4, 5}
assert 0..10 in 5
assert 0..<10 notin 10
assert 0..9 == 0..<10
```

## 浮動小数点数オブジェクト(Float Object)

```python
assert 0.0f64 == 0
assert 0.0f32 == 0.0f64
```

`Ratio`オブジェクトに`Float 64`の単位オブジェクトである`f64`を乗算したものです。

## 複素数オブジェクト(Complex Object)

```python
1+2im, 0.4-1.2im, 0im, im
```

`Complex`オブジェクトは、単に虚数単位オブジェクトである`im`との演算の組み合わせで表します。

## *-less multiplication

Ergでは、解釈に紛れがない限り乗算を表す`*`を省略できます。
ただし、演算子の結合強度は`*`よりも強く設定されています。

```python
# same as `assert (1*m) / (1*s) == 1*(m/s)`
assert 1m / 1s == 1 (m/s)
```

<p align='center'>
    <a href='./00_basic.md'>Previous</a> | <a href='./02_name.md'>Next</a>
</p>
