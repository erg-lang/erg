# Newtype pattern

Here is the Erg version of the newtype pattern commonly used in Rust.

Erg allows type aliases to be defined as follows, but they only refer to the same type.

```python
UserId = Int
```

So, for example, if you have a specification that a number of type `UserId` must be a positive 8-digit number, you can put in `10` or `-1`, because it is the same as type `Int`. If you set it to `Nat`, `-1` can be rejected, but the nature of an 8-digit number cannot be expressed by Erg's type system alone.

Also, for example, when designing a database system, suppose there are several types of IDs: user IDs, product IDs, product IDs, and user IDs. If the number of ID types increases, such as user IDs, product IDs, order IDs, etc., a bug may occur in which different types of IDs are passed to different functions. Even if user IDs and product IDs are structurally equivalent, they are semantically different.

The newtype pattern is a good design pattern for such cases.

```python
UserId = Class {id = Nat}
UserId.
    new id: Nat =
        assert id.dights().len() == 8, else: "UserId must be a positive number with length 8"
        UserId::__new__ {id;}

i = UserId.new(10000000)
print! i # <__main__.UserId object>
i + UserId.new(10000001) # TypeError: + is not implemented between `UserId` and `UserId
```

The constructor guarantees the pre-condition of an 8-digit number.
The `UserId` loses all the methods that `Nat` has, so you have to redefine the necessary operations each time.
If the cost of redefinition is not worth it, it is better to use inheritance. On the other hand, there are cases where the loss of methods is desirable, so choose the appropriate method depending on the situation.
