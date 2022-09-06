# Str

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Str.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Str.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

(不变长度)表示字符串的类型。 简单的 `Str` 类型是删除了字符数的 `StrWithLen N` 类型(`Str = StrWithLen _`)

## 方法

* isnumeric

返回字符串是否为阿拉伯数字。 使用 `isunicodenumeric` 判断汉字数字和其他表示数字的字符(注意此行为与 Python 不同)。