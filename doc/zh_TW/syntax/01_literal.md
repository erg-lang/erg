# 字面量

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/01_literal.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/01_literal.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

## 基本字面量

### 整數字面量

```python
0, -0, 1, -1, 2, -2, 3, -3, ...
```

### 比率文字

```python
0.00, -0.0, 0.1, 400.104, ...
```

如果"比率"文字的整數或小數部分為`0`，則可以省略`0`

```python
assert 1.0 == 1.
assert 0.5 == .5
```

> __注意__: 這個函數 `assert` 用于表明 `1.0` 和 `1.` 相等
后續文檔可能會使用 `assert` 來表示結果是相等的

### 字符串字面量

可以使用任何 Unicode 可表示的字符串
與 Python 不同，引號不能包含在 `'` 中。 如果要在字符串中使用 `"`，請使用 `\"`

```python
"", "a", "abc", "111", "1# 3f2-3*8$", "こんにちは", "?????????? ??????????", ...
```

`{}` 允許您在字符串中嵌入表達式。 這稱為字符串插值
如果要輸出 `{`、`}` 本身，請使用 `\{`、`\}`

```python
assert "1 + 1 is 2" == "{1} + {1} is {1+1}"
s = "1+1"
assert "\{1+1}\" == "\{{s}\}"
```

### 指數字面量

這是學術計算中常用的表示指數符號的文字。 它是"比率"類型的一個實例
該符號與 Python 中的符號相同

```python
1e-34, 0.4e-10, 2.455+e5, 245e5, 25E5, ...
```

```python
assert 1e-10 == 0.0000000001
```

## 復合字面量

這些文字中的每一個都有自己的文檔分別描述它們，因此請參閱該文檔以獲取詳細信息

### [數組字面量](./10_array.md)

```python
[], [1], [1, 2, 3], ["1", "2",], [1, "1", True, [1]], ...
```

### [元組字面量](./11_tuple.md)

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

與 `Array` 字面量不同的是，`Set` 中刪除了重復元素

```python
assert {1, 2, 1} == {1, 2}
```

### 看起來像文字但不是

## 布爾對象

```python
True, False
```

### None 對象

```python
None
```

## Range 對象

```python
assert 0..5 == {1, 2, 3, 4, 5}
assert 0..10 in 5
assert 0..<10 notin 10
assert 0..9 == 0..<10
```

## Float 對象

```python
assert 0.0f64 == 0
assert 0.0f32 == 0.0f64
```

浮點對象是通過將 `Ratio` 對象乘以 `f64` 構造的，后者是 `Float 64` 單位對象

## Complex 對象

```python
1+2im, 0.4-1.2im, 0im, im
```

一個"復雜"對象只是一個虛數單位對象`im`的算術組合

## *-less 乘法

在 Erg 中，您可以省略 `*` 來表示乘法，只要解釋上沒有混淆即可。 但是，運算符的組合強度設置為強于 `*`

```python
# same as `assert (1*m) / (1*s) == 1*(m/s)`
assert 1m / 1s == 1 (m/s)
```

<p align='center'>
    <a href='./00_basic.md'>上一頁</a> | <a href='./02_name.md'>下一頁</a>
</p>
