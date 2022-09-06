# Interval Type

The most basic use of `Range` objects is as iterator.

```python
for! 0..9, i =>
    print! i
```

Note that unlike Python, it includes a end number.

However, this is not only use for the `Range` objects. It can also be used the type. Such a type is called the Interval type.

```python
i: 0..10 = 2
```

The `Nat` type is equivalent to `0..<Inf` and, `Int` and `Ratio` are equivalent to `-Inf<..<Inf`,
`0..<Inf` can also be written `0.._`. `_` means any instance of `Int` type.

Since it is can also be used as iterator, it can be specified in reverse order, such as `10..0`, however `<..`, `..<` and `<..<` cannot be reversed.

```python
a = 0..10 # OK
b = 0..<10 # OK
c = 10..0 # OK
d = 10<..0 # Syntax error
e = 10..<0 # Syntax error
f = 10<..<0 # Syntax error
```

A Range operator can be used for non-numeric types, as long as they are `Ord` immutable types.

```python
Alphabet = "A".."z"
```
