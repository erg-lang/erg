# Subroutine Signatures

## Func

```python
some_func(x: T, y: U) -> V
some_func: (T, U) -> V
```

## Proc

```python
some_proc!(x: T, y: U) => V
some_proc!: (T, U) => V
```

## Func Method

The method type cannot be specified externally with ``Self``.

```python
.some_method(self, x: T, y: U) => ()
# Self.(T, U) => () takes ownership of self
.some_method: Ref(Self). (T, U) => ()
```

## Proc Method (dependent)

In the following, assume that the type `T!` takes the type argument `N: Nat`. To specify it externally, use a type variable.

```python
T!: Nat -> Type
# ~> indicates the state of the type argument before and after application (in this case, self must be a variable reference)
T!(N).some_method!: (Ref! T!(N ~> N+X), X: Nat) => ()
```

As a note, the type of `.some_method` is `Ref!(T(N ~> N+X)). ({X}) => () | N, X: Nat`.
For 方法 that do not have `ref!`, i.e., are deprived of ownership after application, the type argument transition (`~>`) cannot be used.

If ownership is taken, it is as follows.

```python
# If you don't use N, you can omit it with _.
# .some_method!: |N, X: Nat| T!(N).({X}) => T!(N+X)
.some_method!|N, X: Nat|(self(N), X: Nat) => T!(N+X)
```

## Operator

It can be defined as a normal function by enclosing it with ``.

Neuter alphabetic operators such as `and` and `or` can be defined as neuter operators by enclosing them with ``.

```python
and(x, y, z) = x and y and z
`_+_`(x: Foo, y: Foo) = x.a + y.a
`-_`(x: Foo) = Foo.new(-x.a)
```

<p align='center'>
    <a href='./21_lambda.md'>Previous</a> | <a href='./23_closure.md'>Next</a>
</p>
