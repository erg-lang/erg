# Str

（不变长度）表示字符串的类型。 简单的 `Str` 类型是删除了字符数的 `StrWithLen N` 类型（`Str = StrWithLen _`）

## 方法

* isnumeric

返回字符串是否为阿拉伯数字。 使用 `isunicodenumeric` 判断汉字数字和其他表示数字的字符（注意此行为与 Python 不同）。