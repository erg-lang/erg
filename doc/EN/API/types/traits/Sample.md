# Sample

A trait that has a `sample` and `sample!` method that "randomly" picks an instance. The `sample` method always returns the same instance, and the `sample!` method returns a random instance that changes from call to call.

Note that this is a trait that assumes that you want an appropriate instance for testing, etc., and that it is not necessarily random. If you want random sampling, use the `random` module.

All major value classes implement `Sample`. It is also implemented in tuple types, record types, Or types, and sieve types that are composed of `Sample` classes.

``` erg
assert Int. sample() == 42
assert Str. sample() == "example"
# Int is sampled in 64bit range by default
print! Int. sample!() # 1313798
print! {x = Int; y = Int}.sample!() # {x = -32432892, y = 78458576891}
```

Below is an implementation example of `Sample`.

``` erg
EmailAddress = Class {header = Str; domain = Str}, Impl=Sample and Show
@Impl Show
Email address.
    show self = "{self::header}@{self::domain}"
@Impl Sample
Email address.
    sample(): Self = Self.new "sample@gmail.com"
    sample!(): Self =
        domain = ["gmail.com", "icloud.com", "yahoo.com", "outlook.com", ...].sample!()
        header = AsciiStr. sample!()
        Self. new {header; domain}
```