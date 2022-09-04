# module `unsound`

提供的api執行在Erg的類型系統中無法保證安全的不健全和不安全的操作。

## `unsafe!`

執行一個`Unsafe`過程。就像Rust一樣，`Unsafe`的api不能被直接調用，而是全部作為高階函數傳遞給這個過程。

``` erg
unsound = import "unsound"

i = unsound.unsafe!do !:
# convert `Result Int` to `Int`
unsound.transmute input !() . try into (int), int
```

## transmute

將第一個參數的對象轉換成第二個參數的類型。不進行模板檢查。
這個函數損害型系統的型安全性。使用的時候請進行驗證等。

## auto_transmute

與`transmute`不同，自動轉換為期待的類型。與Ocaml的`Obj.magic`作用相同。