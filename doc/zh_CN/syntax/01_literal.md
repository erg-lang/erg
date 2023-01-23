# 字面量

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/01_literal.md%26commit_hash%3D2284988386db7516d7f2d0cb25b27bd8397dd69e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/01_literal.md&commit_hash=2284988386db7516d7f2d0cb25b27bd8397dd69e)

## 基本字面量

### 整数字面量

```python
0, -0, 1, -1, 2, -2, 3, -3, ...
```

### 比率文字

```python
0.00, -0.0, 0.1, 400.104, ...
```

注意，Ratio类型不同于Float类型，虽然API相同，但计算结果的准确性和效率存在差异。

如果"比率"文字的整数或小数部分为`0`，则可以省略`0`

```python
assert 1.0 == 1.
assert 0.5 == .5
```

> __注意__: 这个函数 `assert` 用于表明 `1.0` 和 `1.` 相等
后续文档可能会使用 `assert` 来表示结果是相等的

### 字符串字面量

可以使用任何 Unicode 可表示的字符串
与 Python 不同，引号不能包含在 `'` 中。如果要在字符串中使用 `"`，请使用 `\"`

```python
"", "a", "abc", "111", "1# 3f2-3*8$", "こんにちは", "السَّلَامُ عَلَيْكُمْ", ...
```

`\{..}`允许您在字符串中嵌入表达式。这称为字符串插值
如果要输出`\{..}`本身，请使用`\\{..}`

```python
assert "1 + 1 is 2" == "\{1} + \{1} is \{1+1}"
```

文档注释也被视为字符串字面量，因此可以使用字符串插值。
它在编译时展开。如果您嵌入的表达式在编译时无法确定，则会收到警告。

```python
PI = 3.14159265358979323
'''
S(r) = 4 × \{PI} × r^2
'''
sphere_surface r = 4 * PI * r ** 2
```

### 指数字面量

这是学术计算中常用的表示指数符号的文字。它是"比率"类型的一个实例
该符号与 Python 中的符号相同

```python
1e-34, 0.4e-10, 2.455+e5, 245e5, 25E5, ...
```

```python
assert 1e-10 == 0.0000000001
```

## 复合字面量

这些文字中的每一个都有自己的文档分别描述它们，因此请参阅该文档以获取详细信息

### [数组字面量](./10_array.md)

```python
[], [1], [1, 2, 3], ["1", "2",], [1, "1", True, [1]], ...
```

### [元组字面量](./11_tuple.md)

```python
(), (1, 2, 3), (1, "hello", True), ...
```

### [字典字面量](./12_dict.md)

```python
{:}, {"one": 1}, {"one": 1, "two": 2}, {"1": 1, "2": 2}, {1: "1", 2: True, "three": [1]}, ...
```

### [Record 字面量](./13_record.md)

```python
{=}, {one = 1}, {one = 1; two = 2}, {.name = "John"; .age = 12}, {.name = Str; .age = Nat}, ...
```

### [Set 字面量](./14_set.md)

```python
{}, {1}, {1, 2, 3}, {"1", "2", "1"}, {1, "1", True, [1]} ...
```

与 `Array` 字面量不同的是，`Set` 中删除了重复元素

```python
assert {1, 2, 1} == {1, 2}
```

### 看起来像文字但不是

## 布尔对象

```python
True, False
```

### None 对象

```python
None
```

## Range 对象

```python
assert 0..5 == {1, 2, 3, 4, 5}
assert 0..10 in 5
assert 0..<10 notin 10
assert 0..9 == 0..<10
```

## Float 对象

```python
assert 0.0f64 == 0
assert 0.0f32 == 0.0f64
```

浮点对象是通过将 `Ratio` 对象乘以 `f64` 构造的，后者是 `Float 64` 单位对象

## Complex 对象

```python
1+2Im, 0.4-1.2Im, 0Im, Im
```

一个"复杂"对象只是一个虚数单位对象`Im`的算术组合

## *-less 乘法

在 Erg 中，您可以省略 `*` 来表示乘法，只要解释上没有混淆即可。但是，运算符的组合强度设置为强于 `*`

```python
# same as `assert (1*m) / (1*s) == 1*(m/s)`
assert 1m / 1s == 1 (m/s)
```

<p align='center'>
    <a href='./00_basic.md'>上一页</a> | <a href='./02_name.md'>下一页</a>
</p>
