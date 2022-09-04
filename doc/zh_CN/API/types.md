# List of built-in Erg types

Attributes of the type itself are not stored in the `.__dict__` and cannot be referenced from the instance

## Fundamental types

### Objects

* `__dir__`: Returns the attributes of the object as an array (dir function)
* `__getattribute__`: get and return an attribute
* `__hash__`: returns the hash value of the object
* `__repr__`: string representation of the object (not rich/default implementation exists)
* `__sizeof__`: returns the size of the object (including the size allocated in the heap)

### Show

* `__str__`: returns the string representation (rich) of the object

###Fmt

* `__format__`: Returns a formatted string

### Doc

* `__doc__`: object description

### Named

* `__name__`: the name of the object

### Pickles

* `__reduce__`: Serialize objects with Pickle
* `__reduce_ex__`: __reduce__ that allows you to specify the protocol version

## Object system

Trait class is equivalent to ABC (abstract base class, interface) in Python
Instance belongs to 1, True, "aaa", etc.
Class is Int, Bool, Str, etc.

### Type

* `__supers__`: Supertypes (`__mro__` is an array, but this one is a Set)
* `__basicsize__`:
* `__dictoffset__`: not supported by Evm
* `__flags__`:
* `__itemsize__`: Size of instance (0 if not Class)
* `__weakrefoffset__`: not supported by Evm
* `__membercheck__`: equivalent to `ismember(x, T)`
* `__subtypecheck__`: Equivalent to `issubtype(U, T)`, with alias `__subclasshook__` (compatible with CPython)

### Instances

* `__class__`: Returns the class from which the instance was created (automatically attached to objects created with `.new`)

### Class

* `__mro__`: Type array for method resolution (includes itself, always ends with Object)
* `__base__`: base type (`__mro__[1]` if there are multiple)
* `__new__`: instantiate
* `__init__`: Initialize the instance
* `__init_subclass__`: Initialize the instance
* `__intstancecheck__`: use like `MyClass.__instancecheck__(x)`, equivalent to `isinstance(x, MyClass)`
* `__subclasscheck__`: equivalent to `issubclass(C, MyClass)`

## operator

Operators other than those specified here have no special types

### Eq

* `__eq__(self, rhs: Self) -> Bool`: object comparison function (==)
* `__ne__`: object comparison function (!=), with default implementation

### Ord

* `__lt__(self, rhs: Self) -> Bool`: Object comparison function (<)
* `__le__`: object comparison function (<=), with default implementation
* `__gt__`: object comparison function (>), with default implementation
* `__ge__`: object comparison function (>=), with default implementation

### Bin Add

* Implements `__add__(self, rhs: Self) -> Self`: `+`

### Add R

* `__add__(self, rhs: R) -> Self.AddO`

### Sub R

* `__sub__(self, rhs: R) -> Self.SubO`

### Mul R

* `__mul__(self, rhs: R) -> Self.MulO`

### BinMul <: Mul Self

* `__pow__`: implements `**` (with default implementation)

### Div R, O

* Implements `__div__(self, rhs: Self) -> Self`: `/`, may panic due to 0

### BinDiv <: Div Self

* `__mod__`: implement `%` (with default implementation)

## numeric type

### Num (= Add and Sub and Mul and Eq)

As an example other than Complex, Vector, Matrix, and Tensor are Num (* in Matrix and Tensor are the same as dot and product, respectively)

### Complex (= Inherit(Object, Impl := Num))

* `imag: Ratio`: returns the imaginary part
* `real: Ratio`: returns the real part
* `conjugate self -> Complex`: returns the complex conjugate

### Float (= Inherit(FloatComplex, Impl := Num))

### Ratio (= Inherit(Complex, Impl := Num))

* `numerator: Int`: returns the numerator
* `denominator: Int`: Returns the denominator

### Int (= Inherit Ratio)

### Nat (= Inherit Int)

* `times!`: run the proc self times

## Other basic types

### Bool

* `__and__`:
* `__or__`:
* `not`:

## Str (<: Seq)

* `capitalize`
* `chomp`: remove newline characters
* `isalnum`:
* `isascii`:
* `isalpha`:
* `isdecimal`:
* `is sight`:
* `is identifier`
*`islower`
* `is numeric`
* `isprintable`
* `isspace`
* `is title`
* `isupper`
*`lower`
* `swapcase`
* `title`
* `upper`

## others

### Bits

* `from_bytes`: Convert from Bytes
* `to_bytes`: Convert to Bytes (specify length and endian (byteorder))
* `bit_length`: returns bit length

### Iterable T

Note that it is not the type of `Iterator` itself. `Nat` is `Iterable` but you can't `Nat.next()`, you need to `Nat.iter().next()`.

* `iter`: Create an Iterator.

### Iterator T

Nat and Range have Iterators, so `Nat.iter().map n -> n**2`, `(3..10).iter().fold (sum, n) -> sum + n*2` etc. are possible.
Since all and any are destroyed after use, there are no side effects. These are supposed to be implemented using `next` which has no side effects, but internally `Iterator!.next!` is used for execution efficiency.

* `next`: Returns the first element and the remaining Iterator.
*`all`
*`any`
*`filter`
* `filter_map`
* `find`
* `find_map`
* `flat_map`
* `flatten`
* `fold`
* `for_each`
*`map`
* `map_while`
* `nth`
*`pos`
* `take`
* `unzip`
*`zip`

### Iterator!T = IteratorT and ...

* `next!`: Get the first element.

## SizedIterator T = Iterator T and ...

An Iterator over a finite number of elements.

* `len`:
* `chain`:
* `count`:
* `is_empty`:
* `rev`:
* `next_back`:
* `nth_back`:
* `rfind`:
* `rfold`:
* `sum`:
* `max`:
* `min`:

## Seq T = SizedIterable T and ...

* `concat`: Combine two Seqs
* `__getitem__`: Equivalent to accessing with `[]` (otherwise panics)
* Unlike `get`: __getitem__, it returns Option
* `maketrans`: Create a replacement table (static method)
* `replace`: replace
* `translate`: replace according to the replacement table
* `insert`: Add to idx
* `remove`: remove idx
* `prepend`: prepend
* `dequeue`: remove the head
* `push`: added to the end
* `pop`: take the tail
* `dedup`: remove consecutive values
* `uniq`: Remove duplicate elements (implemented by sort |> dedup, so order may change)
* `swap`: Swap elements
* `reverse`: reverse elements
* `sort`: sort elements
* `first`:
* `last`:

### Seq! T (= Seq T and ...)

* `__setitem__!`:
* `__delitem__!`:
* `insert!`: Add to idx
* `remove!`: remove idx
* `prepend!`: prepend
* `dequeue!`: remove the beginning
* `push!`: added to the end
* `pop!`: take the tail
* `dedup!`: remove consecutive values
* `uniq!`: Remove duplicate elements (implemented by sort! |> dedup!, so order may change)
* `swap!`: swap elements
* `reverse!`: reverse the element
* `set!`
* `sort!`: sort elements
* `translate!`