# module `repl`

provides REPL(Read-Eval-Print-Loop)-related APIs。

## functions

* ` gui _ help `

在浏览器中显示关于对象的信息。离线也可以使用。

## types

### Guess = Object

## ## methods

* * ` . guess `

根据所给出的自变量和返回值来推测函数。

``` erg
guess((1，)， 2) # <int. __add__method=""></int.>
[1, 2] . guess(3、4),[1,2,3,4])# < array (t, n) . concat method >
```