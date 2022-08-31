# Interval Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/10_interval.md%26commit_hash%3D2f89a30335024a46ec0b3f6acc6d5a4b8238b7b0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/10_interval.md&commit_hash=2f89a30335024a46ec0b3f6acc6d5a4b8238b7b0)

The most basic use of `Range` objects is as iterator.

```erg
for! 0..9, i =>
    print! i
```

Note that unlike Python, it includes a end number.

However, this is not only use for the `Range` objects. It can also be used the type. Such a type is called the Interval type.

```erg
i: 0..10 = 2
```

The `Nat` type is equivalent to `0..<Inf` and, `Int` and `Ratio` are equivalent to `-Inf<..<Inf`,
`0..<Inf` can also be written `0.._`. `_` means any instance of `Int` type.

Since it is can also be used as iterator, it can be specified in reverse order, such as `10..0`, however `<..`, `..<` and `<..<` cannot be reversed.

```erg
a = 0..10 # OK
b = 0..<10 # OK
c = 10..0 # OK
d = 10<..0 # Syntax error
e = 10..<0 # Syntax error
f = 10<..<0 # Syntax error
```

A Range operator can be used for non-numeric types, as long as they are `Ord` immutable types.

```erg
Alphabet = "A".."z"
```
