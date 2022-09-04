# Inheritance

Inheritance allows you to define a new class that adds functionality or specialization to an existing class.
Inheritance is similar to inclusion in a trait. The inherited class becomes a subtype of the original class.

```python
NewInt = Inherit Int
NewInt.
    plus1 self = self + 1

assert NewInt.new(1).plus1() == 2
assert NewInt.new(1) + NewInt.new(1) == 2
```

If you want the newly defined class to be inheritable, you must give it the `Inheritable` decorator.

You can specify an optional argument `additional` to allow the class to have additional instance attributes, but only if the class is a value class. However, you cannot add instance attributes if the class is a value class.

```python
@Inheritable
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}

john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

MailAddress = Inherit Str, additional: {owner = Str} # TypeError: instance variables cannot be added to a value class
```

Erg is exceptionally designed not to allow inheritance of type ``Never``. Erg is exceptionally designed not to allow inheritance of `Never` type, because `Never` is a unique class that can never be instantiated.

## Inheritance of Enumerated Classes

[Or type](./13_algebraic.md) can also be inherited. In this case, you can remove any of the choices (multiple choices are possible with `or`) by specifying the optional argument `Excluding`.
No additional choices can be added. The class to which you add an option is not a subtype of the original class.

```python
Number = Class Int or Float or Complex
Number.abs(self): Float =
    match self:
        i: Int -> i.abs().into Float
        f: Float -> f.abs()
        c: Complex -> c.abs().into Float

# c: Complex cannot appear in match choices
RealNumber = Inherit Number, Excluding: Complex
```

Similarly, [refinement type](./12_refinement.md) can also be specified.

```python
Months = Class 0..12
MonthsNot31Days = Inherit Months, Excluding: {1, 3, 5, 7, 8, 10, 12}

StrMoreThan3 = Class StrWithLen N | N >= 3
StrMoreThan4 = Inherit StrMoreThan3, Excluding: StrWithLen N | N == 3
```

## Overriding

The class is the same as the patch in that new 方法 can be added to the original type, but the class can be further "overridden".
This overriding is called override. To override, three conditions must be met.
First, the override must have an `Override` decorator because by default it will result in an error.
In addition, the override cannot change the type of the method. It must be a subtype of the original type.
And if you override a method that is referenced by another method, you must also override all referenced 方法.

Why is this condition necessary? It is because overriding does not merely change the behavior of one method, but may affect the behavior of another method.

Let's start with the first condition. This condition is to prevent "accidental overrides.
In other words, the `Override` decorator must be used to prevent the name of a newly defined method in a derived class from conflicting with the name of the base class.

Next, consider the second condition. This is for type consistency. Since the derived class is a subtype of the base class, its behavior must also be compatible with that of the base class.

Finally, consider the third condition. This condition is unique to Erg and not often found in other object-oriented languages, again for safety. Let's look at what could go wrong if this were not the case.

```python
# Bad example
@Inheritable
Base! = Class {x = Int!}
Base!
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!
    @Override
    g! ref! self = self.f!() # InfiniteRecursionWarning: This code falls into an infinite loop
    # OverrideError: method `.g` is referenced by `.f` but not overridden
```

In the inherited class `Inherited!`, the `.g!` method is overridden to transfer processing to `.f!`. However, the `.f!` method in the base class transfers its processing to `.g!`, resulting in an infinite loop. `.f` was a problem-free method in the `Base!` class, but it was used in an unexpected way by the override, and it was broken.

Erg has built this rule into the specification.

```python
# OK.
@Inheritable
Base! = Class {x = Int!}
Base!
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!
    @Override
    f! ref! self =
        print! self::x
        self::x.update! x -> x + 1
    @Override
    g! ref! self = self.f!()
```

However, this specification does not completely solve the override problem. However, this specification does not completely solve the override problem, since the compiler cannot detect if the override fixes the problem.
It is the responsibility of the programmer creating the derived class to correct the effects of the override. Whenever possible, try to define an alias method.

### Replacing Traits (or what looks like it)

Although it is not possible to replace traits at inheritance time, there are examples that appear to do so.

For example, `Int`, a subtype of `Real` (which implements `Add()`), appears to reimplement `Add()`.

```python
Int = Class ... , Impl := Add() and ...
```

But in fact `Add()` in `Real` stands for `Add(Real, Real)`, and in `Int` it is just overwritten by `Add(Int, Int)`.
They are two different traits (`Add` is a [covariate](./advanced/variance.md), so `Add(Real, Real) :> Add(Int, Int)`).

## Multiple Inheritance

Erg does not allow intersection, diff, and complement between normal classes.

```python
Int and Str # TypeError: cannot unite classes
```

This rule prevents inheritance from multiple classes, i.e., multiple inheritance.

```python
IntAndStr = Inherit Int and Str # SyntaxError: multiple inheritance of classes is not allowed
```

However, multiple inherited Python classes can be used.

## Multi-layer (multi-level) Inheritance

Erg inheritance also prohibits multi-layer inheritance. That is, you cannot define a class that inherits from another class.
Inheritable classes that inherit from an `Object` may exceptionally inherit.

Also in this case, Python's multi-layered inherited classes can be used.

## Rewriting Inherited Attributes

Erg does not allow rewriting the attributes inherited from the base class. This has two implications.

The first is an update operation on the inherited source class attribute. It cannot be reassigned, nor can it be updated by the `.update!` method, for example.

Overriding is different from rewriting because it is an operation to override with a more specialized method. Overrides must also be replaced by compatible types.

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!
    var = !1
    inc_pub! ref! self = self.pub.update! p -> p + 1

Inherited! = Inherit Base!
Inherited!
    var.update! v -> v + 1
    # TypeError: can't update base class variables
    @Override
    inc_pub! ref! self = self.pub + 1
    # OverrideError: `.inc_pub!` must be subtype of `Self! () => ()`
```

The second is an update operation on the (variable) instance attribute of the inherited source. This is also prohibited. Instance attributes of the base class may only be updated from 方法 provided by the base class.
Regardless of the visibility of the attribute, it cannot be updated directly. However, they can be read.

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!
    inc_pub! ref! self = self.pub.update! p -> p + 1
    inc_pri! ref! self = self::pri.update! p -> p + 1

self = self.pub.update!
Inherited!
    # OK
    add2_pub! ref! self =
        self.inc_pub!()
        self.inc_pub!()
    # NG, `Child` cannot touch `self.pub` and `self::pri`.
    add2_pub! ref! self =
        self.pub.update! p -> p + 2
```

After all, Erg inheritance can only add new attributes and override base class 方法.

## Usage of Inheritance

While inheritance is a powerful feature when used correctly, it also has the drawback that it tends to complicate class dependencies, especially when multiple or multi-layer inheritance is used. Complicated dependencies can reduce code maintainability.
The reason Erg prohibits multiple and multi-layer inheritance is to reduce this risk, and the class patch feature was introduced to reduce the complexity of dependencies while retaining the "add functionality" aspect of inheritance.

So, conversely, where should inheritance be used? One indicator is when "semantic subtypes of the base class are desired.
Erg allows the type system to automatically do part of the subtype determination (e.g., Nat, where Int is greater than or equal to 0).
However, for example, it is difficult to create a "string type representing a valid e-mail address" relying solely on Erg's type system. You should probably perform validation on a normal string. Then, we would like to add some kind of "warrant" to the string object that has passed validation. That is the equivalent of downcasting to an inherited class. Downcasting a `Str object` to `ValidMailAddressStr` is a one-to-one correspondence with validating that the string is in the correct email address format.

```python
ValidMailAddressStr = Inherit Str
ValidMailAddressStr.
    init s: Str =
        validate s # mail-address validation
        Self.new s

s1 = "invalid mail address"
s2 = "foo@gmail.com"
_ = ValidMailAddressStr.init s1 # panic: invalid mail address
valid = ValidMailAddressStr.init s2
valid: ValidMailAddressStr # assurance that it is in the correct email address format
```

Another indicator is when you want to achieve a nominal polymorphism.
For example, the `greet!` procedure defined below will accept any object of type `Named`.
But obviously it is wrong to apply a `Dog` type object. So we will use the `Person` class for the argument type.
This way, only `Person` objects, classes that inherit from them, and `Student` objects will be accepted as arguments.
This is more conservative and avoids unnecessarily assuming too much responsibility.

```python
Named = {name = Str; ...}
Dog = Class {name = Str; breed = Str}
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}
structural_greet! person: Named =
    print! "Hello, my name is {person::name}."
greet! person: Person =
    print! "Hello, my name is {person::name}."

max = Dog.new {name = "Max", breed = "Labrador"}
john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

structural_greet! max # Hello, my name is Max.
structural_greet! john # Hello, my name is John.
greet! alice # Hello, my name is Alice.
greet! max # TypeError:
```

<p align='center'>
    <a href='./04_class.md'>Previous</a> | <a href='./06_nst_vs_sst.md'>Next</a>
</p>
