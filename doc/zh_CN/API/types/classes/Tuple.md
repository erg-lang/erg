# Tuple T: *Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Tuple.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Tuple.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

包含多种类型对象的集合

## 方法

* zip self, other

    组合两个有序集合(数组或元组)

    ```python
    assert ([1, 2, 3].zip [4, 5, 6])[0] == (1, 4)
    ```

* zip_by self, op, other

    一种泛化 zip 的方法。您可以指定一个二进制操作来组合
     `()`、`[]`、`{}`、`{:}` 也可以指定为运算符，分别生成元组、数组、集合和字典
    
    ```python
    assert ([1, 2, 3].zip([4, 5, 6]))[0] == (1, 4)
    assert ([1, 2, 3].zip_by((),  [4, 5, 6]))[0] == (1, 4)
    assert ([1, 2, 3].zip_by([],  [4, 5, 6]))[0] == [1, 4]
    assert ([1, 2, 3].zip_by({},  [4, 5, 6]))[0] == {1, 4}
    assert ([1, 2, 3].zip_by({:},  [4, 5, 6]))[0] == {1: 4}
    assert ([1, 2, 3].zip_by(`_+_`, [4, 5, 6]))[0] == 5
    ```
