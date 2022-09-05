# 測試子命令

erg 命令有一個名為 test 的子命令，它支持測試的實現和執行。

## 測試裝飾器 (@Test)

Erg 使用 `erg test` 命令測試包中 `tests` 目錄或 `*.test.er` 文件中的 `@Test` 子例程。
`tests` 子例程負責黑盒測試(不測試私有函數)，`*.test.er` 子例程負責白盒測試(也測試私有函數)。

```python
# tests/test1.er
{add; ...} = import "foo"

@Test
test_1_plus_n(n: Nat) =
    assert add(1, n) == n + 1
```

執行結果以摘要形式顯示，可以以各種文件格式(.md、.csv 等)輸出。

## 文檔測試

在 Erg 中，`#` 和 `#[` 是注釋行，但 `##` 和 `#[[` 是 doc 注釋，并且注釋可以從 VSCode 等編輯器顯示為 markdown。
此外，如果指定了 erg，則使用 erg test 命令自動測試文檔注釋中的源代碼。
下面是一個示例測試。

```python
VMs =...
    ...
    #[[
    execute commands.
    ```python
    # 標準配置的虛擬機
    {vm1; ...} = import "tests/mock"

    assert vm1.exec!("i = 0") == None
    assert vm1.exec!("i").try_into(Int)? == 0
    ```
    ]]#
    .exec! ref self, src =
        ...
    ...
```

用于測試的模擬對象(mock objects)在 `tests/mock` 模塊中定義。