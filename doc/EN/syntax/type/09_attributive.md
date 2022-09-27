# Attributive Type

Attribute types are types that contain Record and Dataclass, Patch, Module, etc.
Types belonging to attribute types are not value types.

## Record Type Composite 

It is possible to flatten Record types composited.
For example, `{... {.name = Str; .age = Nat}; ... {.name = Str; .id = Nat}}` becomes `{.name = Str; .age = Nat; .id = Nat}`.

<p align='center'>
    <a href='./08_value.md'>Previous</a> | <a href='./10_interval.md'>Next</a>
</p>