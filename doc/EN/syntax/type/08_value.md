# Value Type

Value types are Erg built-in types that can be evaluated at compile time, specifically:

```python
Value = (
    Int
    or Nat
    or Ratio
    or Float
    or Complex
    or Bool
    or Str
    or NoneType
    or Array Const
    or Tuple Const
    or Set Const
    or ConstFunc(Const, _)
    or ConstProc(Const, _)
    or ConstMethod(Const, _)
)
```

Value-type objects, constants, and compile-time subroutines applied to them are called __constant expressions__.

```python
1, 1.0, 1+2im, True, None, "aaa", [1, 2, 3], Fib(12)
```

Be careful with subroutines. Subroutines may or may not be value types.
Since the substance of a subroutine is just a pointer, it can be treated as a value [<sup id="f1">1</sup>](#1), but when compiling something that is not a subroutine cannot be used in a constant context. is not a value type because it doesn't make much sense.

Types classified as value types may be added in the future.

---

<span id="1" style="font-size:x-small"><sup>1</sup> The term "value type" in Erg differs from the definition in other languages. There is no concept of memory within pure Erg semantics, and it is incorrect to state that it is a value type because it is placed on the stack, or that it is not a value type because it is actually a pointer. A value type only means that it is a `Value` type or its subtypes. [↩](#f1)</span>

<p align='center'>
    <a href='./07_patch.md'>Previous</a> | <a href='./09_attributive.md'>Next</a>
</p>
