# module `repl`

provides REPL(Read-Eval-Print-Loop)-related APIs.

## functions

* `gui_help`

View information about an object in a browser. Can be used offline.

## types

### Guess = Object

#### methods

* `.guess`

Infers a function given its arguments and return value.

```python
1.guess((1,), 2) # <Int.__add__ method>
[1, 2].guess((3, 4), [1, 2, 3, 4]) # <Array(T, N).concat method>
```