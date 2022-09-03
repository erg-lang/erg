# Ownership System

Since Erg is a language that uses Python as its host language, its method of memory management is dependent on the Python implementation.
Semantically, however, Erg's memory management is different from that of Python. The most noticeable differences appear in the ownership system and the prohibition of circular references.

## Ownership

Erg has an ownership system influenced by Rust.
While Rust's ownership system is generally considered arcane, Erg's has been simplified to make it intuitive.
In Erg, ownership is attached to __mutable objects__, which cannot be referenced after you lose ownership.

```erg
v = [1, 2, 3].into [Int; !3].

push!vec, x =
    vec.push!(x)
    vec.

# ownership of v's contents ([1, 2, 3]) is transferred to w
w = push! v, 4
print! v # error: v was moved
print! w # [1, 2, 3, 4]
```

Ownership transfers occur when an object is passed to a subroutine, for example.
If you wish to retain ownership of the object after passing it to a subroutine, you must do cloning, freezing, or borrowing.
However, as described below, borrowing can only be used in limited situations.

## Cloning

Duplicate an object and transfer ownership of it. This is done by applying the `.clone` method to the real argument.
The cloned object will be exactly the same as the original object, but independent of each other and unaffected by changes.

Cloning is equivalent to deep copying in Python, and is generally more computationally and memory expensive than freezing and borrowing, since it re-creates the same object in its entirety.
Subroutines that require object duplication are called "argument-consuming" subroutines.

```erg
capitalize s: Str!
    s.capitalize!()
    s

s1 = !" hello"
s2 = capitalize s1.clone()
log s2, s1 # !" HELLO hello"
```

## Freezing

Taking advantage of the fact that immutable objects can be referenced from multiple places, a variable object is converted to an immutable object.
This is called freezing. Freezing is used to create iterators from mutable arrays.
Since iterators cannot be created directly from mutable arrays, they are converted to immutable arrays.
If you do not want to destroy the array, use the [`.freeze_map` method](./type/mut.md), etc.

```erg
# Calculate the sum of the values produced by the iterator
sum|T <: Add + HasUnit| i: Iterator T = ...

x = [1, 2, 3].into [Int; !3].
x.push!(4)
i = x.iter() # TypeError: [Int; !4] has no method `iter`.
y = x.freeze()
i = y.iter()
assert sum(i) == 10
y # y is still touched after this.
```

## Borrowing

Borrowing is less expensive than cloning or freezing.
Borrowing can be done in simple cases such as the following.

```erg
peek_str ref(s: Str!) =
    log s

s = !" hello"
peek_str s
```

The borrowed value is called a __reference__ to the original object.
You can "subloan" a reference to another subroutine, but you can't consume it because you are only borrowing it.

```erg
steal_str ref(s: Str!) =.
    # The log function only borrows arguments, so it can subloan
    log s
    # Discard function consumes arguments, so it is an error
    discard s # OwnershipError: cannot consume a borrowed value
    # hint: use `clone` method
```

```erg
steal_str ref(s: Str!) =
    # this is also no good (= consumes the right side)
    x = s # OwnershipError: cannot consume a borrowed value
    x
```

Erg references are more restrictive than Rust. Although references are first-class objects in the language, they cannot be created explicitly and can only be specified as a way of passing real arguments by `ref`/`ref!`.
This means that it is not possible to pack references into arrays or create classes with references as attributes.

However, such restrictions are common in languages without references, so they are not that inconvenient.

## Circular references

Erg is designed to prevent unintentional memory leaks, and the memory checker will generate an error when it detects a circular reference. In most cases, this error can be resolved with a weak reference `Weak`. However, this does not allow the creation of objects with circular structures such as cyclic graphs, so we plan to implement an API that can create circular references as an unsafe operation.

<p align='center'>
    <a href='./17_mutability.md'>Previous</a> | <a href='./19_visibility.md'>Next</a>
</p>
