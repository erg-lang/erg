# Never

It is a subtype of all types. It is a `Class` because it has all the methods and of course `.new`. However, it does not have an instance, and the Erg stops the moment it is about to be created.
There is also a type called `Panic` that does not have an instance, but `Never` is used for normal termination or an intentional infinite loop, and `Panic` is used for abnormal termination.

``` erg
# Never <: Panic
f(): Panic = exit 0 # OK
g(): Never = panic() # TypeError
```

The OR type of `Never`/`Panic`, eg `T or Never` can be converted to `T`. This is because `Never` is a semantically never-occurring option (if it does, the program stops immediately).
However, when using it in the return value type of a function, `or Never` cannot be omitted because it indicates that the program may terminate.