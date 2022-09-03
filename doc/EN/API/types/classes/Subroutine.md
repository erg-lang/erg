# Subroutines

Base type of Func and Proc.

## methods

* return

Interrupts a subroutine and returns the specified value. Useful for quickly escaping from a nest.

``` erg
f x =
     for 0..10, i ->
         if i == 5:
             do
                 f::return i
             do
                 log i
```