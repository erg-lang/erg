# Literal

## Basic Literals

### Int Literal

```erg
0, -0, 1, -1, 2, -2, 3, -3, ...
```

### Ratio Literal

```erg
0.00, -0.0, 0.1, 400.104, ...
```

If a `Ratio` literal has an integer or decimal part of `0`, you can omit the `0`.

```erg
assert 1.0 == 1.
assert 0.5 == .5
```

> __Note__: This function `assert` was used to show that `1.0` and `1.` are equal.
Subsequent documents may use `assert` to indicate that the results are equal.

### Str Literal

Any Unicode-representable string can be used.
Unlike Python, quotation marks cannot be enclosed in `'`. If you want to use `"` in a string, use `\"`.

```erg
"", "a", "abc", "111", "1# 3f2-3*8$", "こんにちは", "السَّلَامُ عَلَيْكُمْ", ...
```

`{}` allows you to embed expressions in strings. This is called string interpolation.
If you want to output `{`, `}` itself, use `\{`, `\}`.

```erg
assert "1 + 1 is 2" == "{1} + {1} is {1+1}"
s = "1+1"
assert "\{1+1}\" == "\{{s}\}"
```

### Exponential Literal

This is a literal representing exponential notation often used in academic calculations. It is an instance of type ``Ratio``.
The notation is the same as in Python.

```erg
1e-34, 0.4e-10, 2.455+e5, 245e5, 25E5, ...
```

```erg
assert 1e-10 == 0.0000000001
```

## Compound Literals

Each of these literals has its own documentation describing them separately, so please refer to that documentation for details.

### [Array Literal](./10_array.md)

```erg
[], [1], [1, 2, 3], ["1", "2",], [1, "1", True, [1]], ...
```

### [Dict Literal](./11_dict.md)

```erg
{:}, {"one": 1}, {"one": 1, "two": 2}, {"1": 1, "2": 2}, {1: "1", 2: True, "three": [1]}, ...
```

### [Tuple Literal](./12_tuple.md)

```erg
(), (1, 2, 3), (1, "hello", True), ...
```

### [Record Literal](./13_record.md)

```erg
{=}, {one = 1}, {one = 1; two = 2}, {.name = "John"; .age = 12}, {.name = Str; .age = Nat}, ...
```

### [Set Literal](./14_set.md)

```erg
{}, {1}, {1, 2, 3}, {"1", "2", "1"}, {1, "1", True, [1]} ...
```

As a difference from `Array` literals, duplicate elements are removed in `Set`.

```erg
assert {1, 2, 1} == {1, 2}
```

### What looks like a literal but isn't

## Boolean Object

```erg
True, False
```

### None Object

```erg
None
```

## Range Object

```erg
assert 0..5 == {1, 2, 3, 4, 5}
assert 0..10 in 5
assert 0..<10 notin 10
assert 0..9 == 0..<10
```

## Float Object

```erg
assert 0.0f64 == 0
assert 0.0f32 == 0.0f64
```

Float objects are constructed by multiplying a `Ratio` object by `f64`, which is a `Float 64` unit object.

## Complex Object

```erg
1+2im, 0.4-1.2im, 0im, im
```

A `Complex` object is simply an arithmetic combination of an imaginary unit object, `im`.

## *-less multiplication

In Erg, you can omit the `*` to indicate multiplication as long as there is no confusion in interpretation. However, the combined strength of the operators is set stronger than `*`.

```erg
# same as `assert (1*m) / (1*s) == 1*(m/s)`
assert 1m / 1s == 1 (m/s)
```

<p align='center'>
    <a href='./00_basic.md'>Previous</a> | <a href='./02_name.md'>Next</a>
</p>
