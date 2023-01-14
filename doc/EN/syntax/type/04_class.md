# Class

A class in Erg is roughly a type that can create its own elements (instances).
Here is an example of a simple class.

```python
Person = Class {.name = Str; .age = Nat}
# If `.new` is not defined, then Erg will create `Person.new = Person::__new__`
Person.
    new name, age = Self::__new__ {.name = name; .age = age}

john = Person.new "John Smith", 25
print! john # <Person object>
print! classof(john) # Person
```

The type given to `Class` (normally a record type) is called the requirement type (in this case `{.name = Str; .age = Nat}`).
Instances can be created with `<Class name>::__new__ {<attribute name> = <value>; ...}` can be created with.
`{.name = "John Smith"; .age = 25}` is just a record, but it is converted to a `Person` instance by passing `Person.new`.
The subroutine that creates such an instance is called a constructor.
In the class above, the `.new` method is defined so that field names, etc. can be omitted.

Note that the following definition without line breaks will result in a syntax error.

```python,compile_fail
Person.new name, age = ... # SyntaxError: cannot define attributes directly on an object
```

## Newtype notation

You can define a class `C` by `C = Class T` for a non-record type `T`. This is a short-hand notation, which is equivalent to `C = Class {base = T}`.
This is to simplify the definition of the so-called "new type pattern".
Also, the constructor `__new__`/`new` can be passed directly to a `T` type object without wrapping it in a record.

```python
Id = Class {base = Int}
i = Id.new {base = 1}
# ↓
Id = Class Int
i = Id.new 1
```

## Instance and class attributes

In Python and other languages, instance attributes are often defined on the block side as follows, but note that such writing has a different meaning in Erg.

```python
# Python
class Person:
    name: str
    age: int
```

```python
# In Erg, this notation implies the declaration of a class attribute (not an instance attribute)
Person = Class()
Person.
    name: Str
    age: Int
```

```python
# Erg code for the Python code above
Person = Class {
    .name = Str
    .age = Nat
}
```

Element attributes (attributes defined in a record) and type attributes (also called instance/class attributes, especially in the case of classes) are completely different things. Type attributes are attributes of the type itself. An element of a type refers to a type attribute when it does not have the desired attribute in itself. An element attribute is a unique attribute directly possessed by the element.
Why is this distinction made? If all attributes were element attributes, it would be inefficient to duplicate and initialize all attributes when the object is created.
In addition, dividing the attributes in this way clarifies roles such as "this attribute is shared" and "this attribute is held separately".

The example below illustrates this. The attribute `species` is common to all instances, so it is more natural to use it as a class attribute. However, the attribute `name` should be an instance attribute because each instance should have it individually.

```python
Person = Class {name = Str}
Person::
    species = "human"
Person.
    describe() =
        log "species: \{species}"
    greet self =
        log "Hello, My name is \{self::name}."

Person.describe() # species: human
Person.greet() # TypeError: unbound method Person.greet needs an argument

john = Person.new {name = "John"}
john.describe() # species: human
john.greet() # Hello, My name is John.

alice = Person.new {name = "Alice"}
alice.describe() # species: human
alice.greet() # Hello, My name is Alice.
```

Incidentally, if an instance attribute and a type attribute have the same name and the same type, a compile error occurs. This is to avoid confusion.

```python
C = Class {.i = Int}
C.i = 1 # AttributeError: `.i` is already defined in instance fields
```

## Class, Type

Note that the class and type of `1` are different.
There is only one class `Int` that is the generator of `1`. You can get the class to which an object belongs by `classof(obj)` or `obj.__class__`.
In contrast, there are countless types of `1`. For example, `{1}, {0, 1}, 0..12, Nat, Int, Num`.
However, the smallest type can be defined as a single type, in this case `{1}`. The type to which an object belongs can be obtained with `Typeof(obj)`. This is a compile-time function.
Objects can use patch methods as well as class methods.
Erg does not allow you to add class methods, but you can use [patch](./07_patch.md) to extend a class.

You can also inherit from existing classes ([Inheritable](../29_decorator.md#inheritable) class).
You can create an inherited class by using `Inherit`. The type on the left-hand side is called the derived class, and the argument type of `Inherit` on the right-hand side is called the base class (inherited class).

```python
MyStr = Inherit Str
# other: You can use MyStr if you set ``other: Str''.
MyStr.
    `-` self, other: Str = self.replace other, ""

abc = MyStr.new("abc")
# Comparison here gets an upcast
assert abc - "b" == "ac"
```

Unlike Python, the defined Erg classes are `final` (non-inheritable) by default.
To make a class inheritable, an `Inheritable` decorator must be attached to the class.
Str` is one of the inheritable classes.

```python
MyStr = Inherit Str # OK
MyStr2 = Inherit MyStr # NG

@Inheritable
InheritableMyStr = Inherit Str
MyStr3 = Inherit InheritableMyStr # OK
```

`Inherit Obj` and `Class()` are almost equivalent in practice. The latter is generally used.

Classes have a different equivalence checking mechanism than types.
Types are equivalence tested based on their structure.

```python
Person = {.name = Str; .age = Nat}
Human = {.name = Str; .age = Nat}

assert Person == Human
```

class has no equivalence relation defined.

```python
Person = Class {.name = Str; .age = Nat}
Human = Class {.name = Str; .age = Nat}

Person == Human # TypeError: cannot compare classes
```

## Difference from structural types

We said that a class is a type that can generate its own elements, but that is not a strict description. In fact, a record type + patch can do the same thing.

```python
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    new name, age = {.name; .age}

john = Person.new("John Smith", 25)
```

There are four advantages to using classes.
The first is that the constructor is validity checked, the second is that it is more performant, the third is that you can use notational subtypes (NSTs), and the fourth is that you can inherit and override.

We saw earlier that record type + patch can also define a constructor (of sorts), but this is of course not a legitimate constructor. This is of course not a legitimate constructor, because it can return a completely unrelated object even if it calls itself `.new`. In the case of a class, `.new` is statically checked to see if it produces an object that satisfies the requirements.

~

Type checking for classes is simply a matter of checking the object's `. __class__` attribute of the object. So it is fast to check if an object belongs to a type.

~

Erg enables NSTs in classes; the advantages of NSTs include robustness.
When writing large programs, it is often the case that the structure of an object is coincidentally matched.

```python
Dog = {.name = Str; .age = Nat}
DogImpl = Patch Dog
DogImpl.
    bark = log "Yelp!"
...
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    greet self = log "Hello, my name is \{self.name}."

john = {.name = "John Smith"; .age = 20}
john.bark() # "Yelp!"
```

The structure of `Dog` and `Person` is exactly the same, but it is obviously nonsense to allow animals to greet and humans to bark.
The former is impossible, so it is safer to make it inapplicable. In such cases, it is better to use classes.

```python
Dog = Class {.name = Str; .age = Nat}
Dog.bark = log "Yelp!"
...
Person = Class {.name = Str; .age = Nat}
Person.greet self = log "Hello, my name is \{self.name}."

john = Person.new {.name = "John Smith"; .age = 20}
john.bark() # TypeError: `Person` object has no method `.bark`.
```

Another feature is that the type attributes added by the patch are virtual and are not held as entities by the implementing class.
That is, `T.x`, `T.bar` are objects that can be accessed (compile-time bound) by types compatible with `{i = Int}`, and are not defined in `{i = Int}` or `C`.
In contrast, class attributes are held by the class itself. Therefore, they cannot be accessed by classes that are not in an inheritance relationship, even if they have the same structure.

```python
C = Class {i = Int}
C.
    foo self = ...
print! dir(C) # ["foo", ...].

T = Patch {i = Int}
T.
    x = 1
    bar self = ...
print! dir(T) # ["bar", "x", ...].
assert T.x == 1
assert {i = 1}.x == 1
print! T.bar # <function bar>
{i = Int}.bar # TypeError: Record({i = Int}) has no method `.bar`.
C.bar # TypeError: C has no method `.bar` print!
print! {i = 1}.bar # <method bar>
C.new({i = 1}).bar # <method bar>
```

## Difference from Data Class

There are two types of classes: regular classes, which are generated with `Class(record)`, and data classes, which are generated with `Inherit(record)`.
The data class inherits the functionality of the record class and has features such as decomposition assignment, `==` and `hash` implemented by default, etc. On the other hand, the data class has its own equivalence relation and format display.
On the other hand, if you want to define your own equivalence relations or formatting displays, you should use the normal class.

```python
C = Class {i = Int}
c = C.new {i = 1}
d = C.new {i = 2}
print! c # <C object>
c == d # TypeError: `==` is not implemented for `C`

D = Inherit {i = Int}
e = D::{i = 1} # same as `e = D.new {i = 1}`
f = D::{i = 2}
print! e # D(i=1)
assert e ! = f
```

## Enum Class

To facilitate defining classes of type `Or`, an `Enum` is provided.

```python
X = Class()
Y = Class()
XorY = Enum X, Y
```

Each type can be accessed as `XorY.X`, `XorY.Y` and the constructor can be obtained as `X.new |> XorY.new`.

```python
x1 = XorY.new X.new()
x2 = (X.new |> XorY.new())()
x3 = (Y.new |> XorY.new())()
assert x1 == x2
assert x1 != x3
```

## Class Relationships

A class is a subtype of a requirement type. methods (including patch methods) of the requirement type can be used in the class.

```python
T = Trait {.foo = Foo}
C = Class(... , impl: T)
C.
    foo = foo
    bar x = ...
assert C < T
assert C.foo == foo
assert not T < C
assert T.foo == Foo
```

<p align='center'>
    <a href='./03_trait.md'>Previous</a> | <a href='./05_inheritance.md'>Next</a>
</p>
