# Div R, O

Use `SafeDiv` if there are no errors due to division by zero.

``` erg
Div R, O = Trait {
     .`/` = Self.(R) -> O or Panic
}
```