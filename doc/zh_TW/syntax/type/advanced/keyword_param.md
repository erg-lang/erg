# 關鍵字參數函數類型


```erg
h(f) = f(y: 1, x: 2)
h: |T: Type|((y: Int, x: Int) -> T) -> T
```

帶關鍵字自變量函數的部分定型規則如下所示。


```erg
((x: T, y: U) -> V) <: ((T, U) -> V)  # x, y are arbitrary keyword parameters
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

這意味著關鍵詞自變量可以刪除或者替換。但是，兩者不能同時進行。也就是說，不能將轉換為<gtr=“5”/>。另外，帶有關鍵詞自變量的只在頂級元組內，排列和嵌套的元組中不帶有關鍵詞自變量。


```erg
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```