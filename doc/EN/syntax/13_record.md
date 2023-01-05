# Record

A record is a collection that combines the properties of a Dict accessed by key and a tuple whose access is inspected at compile time.
If you know JavaScript, think of it as a (more enhanced) kind of object literal notation.

```python
john = {.name = "John"; .age = 21}

assert john.name == "John"
assert john.age == 21
assert john in {.name = Str; .age = Nat}
john["name"] # Error: john is not subscribable
```

The `.name` and `.age` parts are called attributes, and the `"John"` and `21` parts are called attribute values.

The difference from JavaScript object literals is that they are not accessible as strings. That is, attributes are not just strings.
This is because access to the value is determined at compile-time, and because dictionaries and records are different things. In other words, `{"name": "John"}` is a Dict and `{name = "John"}` is a record.
So how should we use dictionaries and records?
In general, we recommend using records. Records have the advantages of being checked at compile-time for the existence of elements and of being able to specify __visibility_.
Specifying visibility is equivalent to specifying public/private in Java and other languages. For details, see [visibility](./19_visibility.md) for details.

```python,compile_fail
a = {x = 1; .y = x + 1}
assert a.y == 2
a.x # AttributeError: x is private
# Hint: declare as `.x`.
```

The above example may seem strange to someone familiar with JavaScript, but simply declaring `x` makes it inaccessible from the outside. `. `. `.

You can also explicitly specify the type of an attribute.

```python
anonymous = {
    .name: Option! Str = "Jane Doe"
    .age = 20
}
anonymous.name.set! "John Doe"
```

A record can also have the method.

```python
o = {
    .i = !0
    .inc! ref! self = self.i.inc!()
}

assert o.i == 0
o.inc!()
assert o.i == 1
```

There is a notable syntax with respect to records. When all the attribute values of a record are classes (not structural types), the record itself behaves as a type with its own attributes as required attributes.
Such a type is called a record type. See the section [Record] for more details.

```python
# record
john = {.name = "John"}
# record type
john: {.name = Str}
Named = {.name = Str}
john: Named

greet! n: Named =
    print! "Hello, I am \{n.name}"
john # "Hello, I am John" print!

Named.name # Str
```

## Deconstructing a record

Records can be deconstructed as follows.

```python
record = {x = 1; y = 2}
{x = a; y = b} = record
assert a == 1
assert b == 2

point = {x = 2; y = 3; z = 4}
match point:
    {x = 0; y = 0; z = 0} -> "origin"
    {x = _; y = 0; z = 0} -> "on the x axis"
    {x = 0; ...} -> "x = 0"
    {x = x; y = y; z = z} -> "(\{x}, \{y}, \{z})"
```

`x = ...` can also be abbreviated to `x` when there is a variable with the same name as the attribute, for example, `x = x` or `x = .x` to `x`, and `.x = .x` or `.x = x` to `.x`.
However, when there is only one attribute, it must be followed by `;` to distinguish it from a set.

```python
x = 1
y = 2
xy = {x; y}
a = 1
b = 2
ab = {.a; .b}
assert ab.a == 1
assert ab.b == 2

record = {x;}
tuple = {x}
assert tuple.1 == 1
```

This syntax can be used to deconstructed a record and assign it to a variable.

```python
# same as `{x = x; y = y} = xy`
{x; y} = xy
assert x == 1
assert y == 2
# same as `{.a = a; .b = b} = ab`
{a; b} = ab
assert a == 1
assert b == 2
```

## Empty Record

An empty record is represented by `{=}`. An empty record is also its own class, like Unit.

```python
empty_record = {=}
empty_record: {=}
# Object: Type = {=}
empty_record: Object
empty_record: Structural {=}
{x = 3; y = 5}: Structural {=}
```

An empty record is different from an empty Dict `{:}` or empty set `{}`. In particular, note that it is the opposite of `{}` in meaning (in Python, `{}` is an empty dictionary, while in Erg it is `!{:}` in Erg).
As an enumerated type, `{}` is an empty type that contains nothing in its elements. The `Never` type is a classification of this type.
Conversely, the record class `{=}` has no required instance attribute, so all objects are elements of it. An `Object` is an alias of this.
An `Object` (a patch of `Object`) is an element of `. __sizeof__` and other very basic provided methods.

```python
AnyPatch = Patch Structural {=}
    . __sizeof__ self = ...
    .clone self = ...
    ...
Never = Class {}
```

Note that no other type or class can be structurally equivalent to the `{}`, `Never` type, and it is an error if the user defines a type with `{}`, `Class {}` on the right side.
This means that, for example, `1..10 or -10. -1`, but `1..10 and -10... -1`. `-1` when it should be `1..10 or -10...-1`, for example.
Also, if you define a type (such as `Int and Str`) that results in a composition `Object`, you will be warned to simply set it to `Object`.

## Instant Block

Erg has another syntax, instant block, which simply returns the last value evaluated. Attributes cannot be retained.

```python
x =
    x = 1
    y = x + 1
    y ** 3
assert x == 8

y =
    .x = 1 # SyntaxError: cannot define an attribute in an entity block
```

## Data Class

A bare record (a record generated by a record literal) must be defined directly in the instance if you try to implement a method on its own.
This is inefficient, and as the number of attributes increases, error messages and the like become difficult to see and use.

```python,compile_fail
john = {
    name = "John Smith"
    age = !20
    .greet! ref self = print! "Hello, my name is \{self::name} and I am \{self::age} years old."
    .inc_age! ref! self = self::age.update! x -> x + 1
}
print! john + 1
# TypeError: + is not implemented for {name = Str; age = Int; .greet! = Ref(Self). () => None; inc_age! = Ref! () => None}, Int
```

So, in such a case, you can inherit a record class. Such a class is called a data class.
This is described in [class](./type/04_class.md).

```python,checker_ignore
Person = Inherit {name = Str; age = Nat}
Person.
    greet! ref self = print! "Hello, my name is \{self::name} and I am \{self::age} years old."
    inc_age! ref! self = self::age.update! x -> x + 1

john = Person.new {name = "John Smith"; age = 20}
print! john + 1
# TypeError: + is not implemented for Person, Int
```

<p align='center'>
    <a href='./12_tuple.md'>Previous</a> | <a href='./14_set.md'>Next</a>
</p>
