# Ownership system

Since Erg is a language that uses Python as the host language, the method of memory management depends on the Python implementation.
But semantically Erg's memory management is different from Python's. A notable difference is in the ownership system and the prohibition of circular references.

## Ownership

Erg has an ownership system inspired by Rust.
Rust's ownership system is generally considered esoteric, but Erg's is simplified to be intuitive.
In Erg, __mutable objects__ are owned and cannot be referenced after ownership is lost.

```python
v = [1, 2, 3].into [Int; !3]

push! vec, x =
    vec.push!(x)
    vec

# The contents of v ([1, 2, 3]) are owned by w
w = push! v, 4
print! v # error: v was moved
print!w # [1, 2, 3, 4]
```

Ownership transfer occurs, for example, when an object is passed to a subroutine.
If you want to still have ownership after giving it away, you'll need to clone, freeze, or borrow.
However, as will be explained later, there are limited situations in which it can be borrowed.

## replication

Duplicate an object and transfer its ownership. It does this by applying the `.clone` method to the actual arguments.
The duplicated object is exactly the same as the original, but independent of each other and unaffected by changes.

Duplication is equivalent to Python's deep copy, and since it recreates the same object entirely, the computation and memory costs are generally higher than freezing and borrowing.
A subroutine that needs to duplicate an object is said to be an "argument consuming" subroutine.

```python
capitalize s: Str!=
    s. capitalize!()
    s

s1 = !"hello"
s2 = capitalize s1.clone()
log s2, s1 # !"HELLO hello"
```

## freeze

We take advantage of the fact that immutable objects can be referenced from multiple places and convert mutable objects to immutable objects.
This is called freezing. Freezing is used, for example, when creating an iterator from a mutable array.
Since you can't create an iterator directly from a mutable array, convert it to an immutable array.
If you don't want to destroy the array, use the [`.freeze_map` method](./type/18_mut.md).

```python
# Compute the sum of the values ​​produced by the iterator
sum|T <: Add + HasUnit| i: Iterator T = ...

x = [1, 2, 3].into [Int; !3]
x.push!(4)
i = x.iter() # TypeError: [Int; !4] has no method `iter`
y = x.freeze()
i = y.iter()
assert sum(i) == 10
y # y can still be touched
```

## borrow

Borrowing is cheaper than duplicating or freezing.
Borrowing can be done in the following simple cases:

```python
peek_str ref(s: Str!) =
    log s

s = !"hello"
peek_str s
```

A borrowed value is called a __reference__ to the original object.
You can "sublease" the reference to another subroutine, but you cannot consume it because you are only borrowing it.

```python
steal_str ref(s: Str!) =
    # Since the log function only borrows the arguments, it can be sub-leased
    log s
    # error because the discard function consumes arguments
    discard s # OwnershipError: cannot consume a borrowed value
    # hint: use `clone` method
```

```python
steal_str ref(s: Str!) =
    # This is no good either (= consumes the right side)
    x = s # OwnershipError: cannot consume a borrowed value
    x
```

Erg's references are more restrictive than Rust's. References are first-class objects in the language, but cannot be created explicitly, they can only be specified as argument passing via `ref`/`ref!`.
This means that you cannot stuff references into arrays or create classes with references as attributes.

However, such restrictions are a natural specification in languages ​​without references in the first place, and they are not so inconvenient.

## circular references

Erg is designed to prevent unintentional memory leaks, and will issue an error if the memory checker detects a circular reference. In most cases, this error can be resolved with a weak reference `Weak`. However, since it is not possible to generate objects with circular structures such as cyclic graphs, we plan to implement an API that can generate circular references as unsafe operations.

<p align='center'>
    <a href='./17_mutability.md'>Previous</a> | <a href='./19_visibility.md'>Next</a>
</p>