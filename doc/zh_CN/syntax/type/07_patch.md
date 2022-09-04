# Patch

Erg does not allow modification of existing types and classes.
This means, it is not possible to define additional 方法 in a class, nor to perform specialization (a language feature that monomorphizes a polymorphically declared type and defines a dedicated method, as in C++).
However, there are many situations where you may want to add feature to an existing type or class, and there is a function called "patching" that allows you to do this.

```python
StrReverse = Patch Str
StrReverse.
    reverse self = self.iter().rev().collect(Str)

assert "abc".reverse() == "cba"
```

The name of the patch should be a straightforward description of the primary functionality to be added.
This way, objects of the type being patched (`Str`) can use the 方法 of the patch (`StrReverse`).
In fact, built-in method `.reverse` is not a method of `Str`, but a method added to `StrRReverse`.

However, patch 方法 have lower precedence than 方法 of the nominal type (class/trait) and cannot override 方法 of existing types.

```python
StrangeInt = Patch Int
StrangeInt.
    `_+_` = Int.`_-_` # AssignError: . `_+_` is already defined in Int
```

If you want to override, you must inherit from the class.
However, it is basically recommended not to override and to define a method with a different name.
Overriding is not very easy to do because of some safety restrictions.

```python
StrangeInt = Inherit Int
StrangeInt.
    # Overriding 方法 must be given Override decorators.
    # In addition, you need to override all Int 方法 that depend on Int.`_+_`.
    @Override
    `_+_` = Super.`_-_` # OverrideError: Int.`_+_` is referenced by ... ````` , so these 方法 must also be overridden
```

## Selecting 修补程序

修补程序 can be defined for a single type, and can be grouped together.

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
StrMultiReplace = Patch(Str)
StrMultiReverse.
    multi_replace self, pattern_and_targets: [(Pattern, Str)] = ...
StrToCamelCase = Patch(Str)
StrToCamelCase.
    to_camel_case self = ...
StrToKebabCase = Patch(Str)
StrToKebabCase.
    to_kebab_case self = ...

StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
```

```python
{StrBoosterPack; ...} = import "foo"

assert "abc".reverse() == "cba"
assert "abc".multi_replace([("a", "A"), ("b", "B")]) == "ABc"
assert "to camel case".to_camel_case() == "toCamelCase"
assert "to kebab case".to_kebab_case() == "to-kebab-case"
```

If multiple 修补程序 are defined, some of them may result in duplicate implementations.

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
# more efficient implementation
StrReverseMk2 = Patch(Str)
StrReverseMk2.
    reverse self = ...

"hello".reverse() # PatchSelectionError: multiple choices of `.reverse`: StrReverse, StrReverseMk2
```

In such a case, you can make it unique by using the __related function__ form instead of the method form.

```python
assert StrReverseMk2.reverse("hello") == "olleh"
```

You can also make it unique by selectively importing.

```python
{StrReverseMk2; ...} = import "foo"

assert "hello".reverse() == "olleh"
```

## Glue Patch

修补程序 can also relate types to each other. The `StrReverse` patch relates `Str` and `Reverse`.
Such a patch is called a __glue patch__.
Because `Str` is a built-in type, a glue patch is necessary for users to retrofit traits.

```python
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch Str, Impl := Reverse
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

Only one glue patch can be defined per type/trait pair.
This is because if multiple glue 修补程序 were "visible" at the same time, it would not be possible to uniquely determine which implementation to choose.
However, you can swap 修补程序 when moving to another scope (module).

```python
NumericStr = Inherit Str
NumericStr.
    ...

NumStrRev = Patch NumericStr, Impl := Reverse
NumStrRev.
    ...
# DuplicatePatchError: NumericStr is already associated with `Reverse`
# hint: `Str` (superclass of `NumericStr`) is associated with `Reverse` by `StrReverse`
```

## Appendix: Relationship to Rust's Trait

Erg 修补程序 are the equivalent of Rust's (retrofitted) `impl` blocks.

```rust
// Rust
trait Reverse {
    fn reverse(self) -> Self;
}

impl Reverse for String {
    fn reverse(self) -> Self {
        self.chars().rev().collect()
    }
}
```

You could say that Rust's traits are features of Erg's traits and 修补程序. This makes Rust's traits sound more convenient, but that is not necessarily the case.

```python
# Erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch(Str, Impl := Reverse)
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

Because the `impl` block is objectized as a patch in Erg, selective inclusion is possible when importing from other modules. As a side-effect, it also allows implementation of external traits to external structures.
Also, syntaxes such as `dyn trait` and `impl trait` are no longer required by the structure type.

```python
# Erg
reversible: [Reverse; 2] = [[1, 2, 3], "hello"]

iter|T|(i: Iterable T): Iterator T = i.iter()
```

```rust
// Rust
let reversible: [Box<dyn Reverse>; 2] = [Box::new([1, 2, 3]), Box::new("hello")];

fn iter<I>(i: I) -> impl Iterator<Item = I::Item> where I: IntoIterator {
    i.into_iter()
}
```

## For-All Patch

A patch can be defined not only for one specific type, but also for "function types in general" and so on.
In this case, the term to which the degree of freedom is to be given is given as an argument (in the case below, `T: Type`). A patch defined in this way is called an all-symmetric patch.
As you can see, an all-symmetric patch is precisely a function that returns a patch, but it can also be considered a patch in its own right.

```python
FnType T: Type = Patch(T -> T)
FnType(T).
    type = T

assert (Int -> Int).type == Int
```

## Structural Patch

In addition, 修补程序 can be defined for any type that satisfies a certain structure.
However, this has a lower priority than nominal 修补程序 and class 方法.

Careful design should be used when defining structural 修补程序, as some properties are lost by extension, such as the following.

```python
# This should not be `Structural`
Norm = Structural Patch {x = Int; y = Int}
Norm.
    norm self = self::x**2 + self::y**2

Point2D = Class {x = Int; y = Int}
assert Point2D.new({x = 1; y = 2}).norm() == 5

Point3D = Class {x = Int; y = Int; z = Int}
assert Point3D.new({x = 1; y = 2; z = 3}).norm() == 14 # AssertionError:
```

<p align='center'>
    <a href='./06_nst_vs_sst.md'>Previous</a> | <a href='./08_value.md'>Next</a>
</p>
