# module `repl`

provides REPL(Read-Eval-Print-Loop)-related APIs。

## functions

* ` gui _ help `

在瀏覽器中顯示關於對象的信息。離線也可以使用。

## types

### Guess = Object

## ## methods

* * ` . guess `

根據所給出的自變量和返回值來推測函數。

``` erg
guess((1，)， 2) # <int. __add__method=""></int.>
[1, 2] . guess(3、4),[1,2,3,4])# < array (t, n) . concat method >
```