# Existential type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/existential.md%26commit_hash%3D417bfcea08ed0e09f715f5d272842510fca8f6dd)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/existential.md&commit_hash=417bfcea08ed0e09f715f5d272842510fca8f6dd)

If there is a for-all type corresponding to ∀, it is natural to assume that there is an existential type corresponding to ∃.
Existential types are not difficult. You already know the existential type, just not consciously aware of it as such.

```erg
T: Trait
f x: T = ...
```

The trait `T` above is used as the existential type.
In contrast, `T` in the lower case is only a trait, and `X` is an for-all type.

```erg
f|X <: T| x: X = ...
```

In fact, the existential type is replaced by an for-all type. So why is there such a thing as an existential type?
First of all, as we saw above, existential types do not involve type variables, which simplifies type specification.
Also, since the type variable can be removed, it is possible to construct a type that would have rank 2 or higher if it were an all-presumptive type.

```erg
show_map f: (|T| T -> T), arr: [Show; _] =
    arr.map x ->
        y = f x
        log y
        y
```

However, as you can see, the existential type forgets or expands the original type, so if you do not want to expand the return type, you must use the for-all type.
Conversely, types that are only taken as arguments and are not relevant to the return value may be written as existential types.

```erg
# id(1): I want it to be Int
id|T|(x: T): T = x
# |S <: Show|(s: S) -> () is redundant
show(s: Show): () = log s
```

By the way, a class is not called an existential type. A class is not called an existential type, because its elemental objects are predefined.
Existential type means any type that satisfies a certain trait, and it is not the place to know what type is actually assigned.
