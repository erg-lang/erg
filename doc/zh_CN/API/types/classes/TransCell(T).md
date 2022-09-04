# TransCell! T: Type!

It is a cell whose contents can be changed for each mold. Since it is a subtype of T type, it also behaves as T type.
It's useful when it's type T at initialization, and it's always type U after a certain point.

``` erg
a = TransCell!.new None
a: TransCell! !NoneType
a.set! 1
a: TransCell! !Int
assert a + 1 == 2
```