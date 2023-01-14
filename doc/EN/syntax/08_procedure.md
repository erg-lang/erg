# Procedures

Procedures mean the functions that allow [side-effect](./07_side_effect.md).
Please refer to [Function](./04_function.md) basically usage or definition.
Add `!` to a function name to define it.

```python
proc!(x: Int!, y: Int!) =
    for! 0..x, i =>
        for 0..y, j =>
            print! i, j
```

Procedures are necessary when dealing with mutable objects, but having a mutable object as an argument does not necessarily make it a procedure.
Here is a function takes a mutable object (not procedure).

```python
peek_str s: Str! = log s

make_proc(x!: (Int => Int)): (Int => Int) = y => x! y
p! = make_proc(x => x)
print! p! 1 # 1
```

Also, procedures and functions are related by `proc :> func`.
Therefore, it is possible to define functions in procedure.
However, note that the reverse is not possible.

```python
proc!(x: Int!) = y -> log x, y # OK
func(x: Int) = y => print! x, y # NG
```

## Binding
Procedures can manipulate mutable variables that are out of scope.
```python
x = ! 0
proc! () =
 x.inc! ()
proc! ()
assert x == 1
```
In this case, 'proc!' has the following type.
```python
proc!: {| x: Int! |} () => ()
```
`{| x: Int! |} The ' part is called the bind column and represents the variable and its type that the procedure operates on.
Binding columns are derived automatically, so you don't need to write them explicitly.
Note that normal procedures can only manipulate predetermined external variables. This means that variables passed in arguments cannot be rewritten.
If you want to do something like that, you need to use procedural methods. Procedural methods can rewrite 'self'.
```python
C! N = Class {arr = [Int; N]!}
C!.
 new() = Self! (0)::__new__ {arr = ![]}
C! (N).
    # push!: {|self: C!( N) ~> C! (N+1)|} (self: RefMut(C!( N)), x: Int) => NoneType
 push! ref! self, x = self.arr.push! (x)
    # pop!: {|self: C!( N) ~> C! (N-1)|} (self: RefMut(C!( N))) => Int
 pop! ref! self = self.arr.pop! ()
c = C!. new()
c.push! (1)
assert c.pop! () ==  1
```

<p align='center'>
    <a href='./07_side_effect.md'>Previous</a> | <a href='./09_builtin_procs.md'>Next</a>
</p>
