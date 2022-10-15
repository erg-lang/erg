# Tuple T: ...Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Tuple.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Tuple.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

包含多種類型對象的集合

## 方法

* zip self, other

    組合兩個有序集合(數組或元組)

    ```python
    assert ([1, 2, 3].zip [4, 5, 6])[0] == (1, 4)
    ```

* zip_by self, op, other

    一種泛化 zip 的方法。您可以指定一個二進制操作來組合
     `()`、`[]`、`{}`、`{:}` 也可以指定為運算符，分別生成元組、數組、集合和字典
    
    ```python
    assert ([1, 2, 3].zip([4, 5, 6]))[0] == (1, 4)
    assert ([1, 2, 3].zip_by((),  [4, 5, 6]))[0] == (1, 4)
    assert ([1, 2, 3].zip_by([],  [4, 5, 6]))[0] == [1, 4]
    assert ([1, 2, 3].zip_by({},  [4, 5, 6]))[0] == {1, 4}
    assert ([1, 2, 3].zip_by({:},  [4, 5, 6]))[0] == {1: 4}
    assert ([1, 2, 3].zip_by(`_+_`, [4, 5, 6]))[0] == 5
    ```
