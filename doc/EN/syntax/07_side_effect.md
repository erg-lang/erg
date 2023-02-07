# Side effects and procedures

We have been neglecting to explain the meaning of the `!`, but now its meaning will finally be revealed. This `!` indicates that this object is a "procedure" with a "side-effect". A procedure is a function with a side-effect.

```python,compile_fail
f x = print! x # EffectError: functions cannot be assigned objects with side effects
# hint: change the name to 'f!'
```

The above code will result in a compile error. This is because you are using a procedure in a function. In such a case, you must define it as a procedure.

```python
p! x = print! x
```

`p!`, `q!`, ... are typical variable names for procedures.
Procedures defined in this way also cannot be used within a function, so side-effects are completely isolated.

## Methods

Functions and procedures each can be methods. Functional methods can only take immutable references to `self`, while procedural methods can take mutable references to `self`.
The `self` is a special parameter, which in the context of a method refers to the calling object itself. The reference `self` cannot be assigned to any other variable.

```python,compile_fail
C!.
    method ref self =
        x = self # OwnershipError: cannot move out 'self'
        x
```

Procedural methods can also take [ownership](./19_ownership.md) of `self`. Remove `ref` or `ref!` from the method definition.

```python,compile_fail
n = 1
s = n.into(Str) # '1'
n # ValueError: n was moved by .into (line 2)
```

Only one procedural methods can have a mutable reference at any given time. In addition, while a mutable reference is taken, no more mutable reference can be taken from the original object. In this sense, `ref!` causes a side-effect on `self`.

Note, however, that it is possible to create (immutable/mutable) references from mutable references. This allows recursion and `print!` of `self` in procedural methods.

```python,checker_ignore
T -> T # OK (move)
T -> Ref T # OK (move)
T => Ref! T # OK (only once)
Ref T -> T # NG
Ref T -> Ref T # OK
Ref T => Ref!
T -> Ref T # NG
T -> Ref T # OK
T => Ref!
```

## Appendix: Strict definition of side-effects

The rules for whether a code has a side-effect or not are not immediately understandable.
Until you can understand them, we recommend that you leave it to the compiler to define them as functions for the time being, and if an error occurs, add `!` to treat them as procedures.
However, for those who want to understand the exact specifications of the language, the following is a more detailed explanation of side-effects.

First, it must be stated that the equivalence of return values is irrelevant with respect to side effects in Erg.
There are procedures that for any given `x` will result in `p!(x) == p!(x)` (e.g. always return `None`), and there are functions that will result in `f(x) ! = f(x)`.

An example of the former is `print!`, and an example of the latter is the following function.

```python
nan _ = Float.NaN
assert nan(1) ! = nan(1)
```

There are also objects, such as classes, for which equivalence determination itself is not possible.

```python,checker_ignore
T = Structural {i = Int}
U = Structural {i = Int}
assert T == U

C = Class {i = Int}
D = Class {i = Int}
assert C == D # TypeError: cannot compare classes
```

Back to the point: the precise definition of "side-effect" in Erg is

* Accessing mutable external information.

"External" generally refers to the outer scope; computer resources that Erg cannot touch and pre-/post-execution information are not included in "external". "Access" includes reading as well as writing.

As an example, consider the `print!` procedure. At first glance, `print!` does not seem to rewrite any variables. But if it were a function, it could rewrite outer variables, for example, with code like this:

```python
camera = import "some_camera_module"
ocr = import "some_ocr_module"

n = 0
_ =
    f x = print x # Suppose we could use print as a function
    f(3.141592)
cam = camera.new() # camera faces PC display
image = cam.shot!()
n = ocr.read_num(image) # n = 3.141592
```

Think of the `camera` module as an external library providing an API for a certain camera product, and `ocr` as a library for OCR (optical character recognition).
The direct side-effect is caused by `cam.shot!()`, but obviously that information is leaked from `f`. Therefore, `print!` cannot be a function by nature.

Nevertheless, there may be cases where you want to temporarily check a value in a function and do not want to add `!` in the related function just for that purpose. In such cases, the `log` function can be used.
`log` prints the value after the entire code has been executed. In this way, side-effects are not propagated.

```python
log "this will be printed after execution"
print! "this will be printed immediately"
# this will be printed immediately
# this will be printed after execution
```

If there is no feedback to the program, or in other words, if no external object can use the internal information, then the "leakage" of the information may be allowed. It is only necessary that the information not be "propagated".

<p align='center'>
    <a href='./06_operator.md'>Previous</a> | <a href='./08_procedure.md'>Next</a>
</p>
