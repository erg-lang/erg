# Str

(不變長度)表示字符串的類型。 簡單的 `Str` 類型是刪除了字符數的 `StrWithLen N` 類型(`Str = StrWithLen _`)

## 方法

* isnumeric

返回字符串是否為阿拉伯數字。 使用 `isunicodenumeric` 判斷漢字數字和其他表示數字的字符(注意此行為與 Python 不同)。