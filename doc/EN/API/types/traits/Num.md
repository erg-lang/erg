# Num

`A <: B` denotes that type A is a subtype declaration of type B. In addition, type A at this time is called a subtype, and type B is called a generalized type (supertype). Furthermore, if `A <: B` then all expressions with type A have the property of type B. This is called subsumption.

The type relations of Erg built-in numeric types are as follows.

- Boolean type (Bool) <: Natural number type (Nat) <: Integer type (Int) <: Rational number type (Ratio) <: Complex number type (Complex)

As a result, when performing numerical calculations, if the type is not specified, each type is upcast (downcast) if it is a subtype.
Exponential literals are a variant of rational literals, and are of the same type.

> __Note__: In the current implementation, the floating-point class does not exist as a separate class, but is implemented the same way as rational literals. In the future, this floating-point class will be implemented again as a separate class for faster computations.
> Also, complex objects are currently implemented using floating-point objects and will be rewritten with rational literals in the future as well.

```python
>>> 1 + 1.0 # Nat(Int)+Ratio is up-casting to Ratio+Ratio
2.0 # Ratio
>>> 10.0 // 2 # Ratio//Nat(Int) is also up-casting to Ratio//Ratio. The result of Ratio//Ratio is Int
5 # Int(Nat)
>>> True == 1.0 # Bool==Ratio is up-casting to Ratio==Ratio
True
```

If types are not specified, they're inferred so that they are up-casting to be of the same type.
In general, downcasting is unsafe and the conversion method is non-trivial.

Casting between classes cannot be redefined later. Only when a superclass is specified by inheritance when the class is defined is it eligible for casting.
Also, traits cannot be partially typed unless they are basically "implemented" at the time of class definition. However, this can only be done with [patch](../../../syntax/type/07_patch.md).

Covariant compound literals, such as array literals, can be cast if they are in an inclusion relationship.
Note, however, that types with non-degenerate cannot be cast in erg even if they are in an inclusion relationship (for details, see [degenerate](../../../syntax/type/advanced/variance.md)).

## definition

```python
Num R = Add(R) and Sub(R) and Mul(R) and Eq
Num = Num Self
```

## supers

Add and Sub and Mul and Eq

## methods

*`abs`