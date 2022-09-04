# procedures

## print!

``` erg
print!(x) -> NoneType
```

   Returns x with a newline.

##debug&excl;

``` erg
debug!(x, type = Info) -> NoneType
```

Debug x with newline (file name, line number, variable name is displayed together). Removed in release mode.
Emoji-capable terminals are prefixed according to type.

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

## for! i: Iterable T, block: T => NoneType

Traverse the iterator with the action of block.

## while! cond: Bool!, block: () => NoneType

Execute block while cond is True.

## Lineno!() -> Nat

## Filename!() -> Str

## Namespace!() -> Str

## Module!() -> Module