# test 子命令

erg 指令中有 test 這個子指令，進行測試安裝以及執行的支援。

## 測試裝飾器（@Test）

Erg 使用命令測試軟件包中的<gtr=“3”/>目錄或<gtr=“4”/>文件中的<gtr=“5”/>子程序。 <gtr=“7”/>子例程負責黑盒測試（不測試私有函數），<gtr=“8”/>子例程負責白盒測試（也測試私有函數）。


```erg
# tests/test1.er
{add; ...} = import "foo"

@Test
test_1_plus_n(n: Nat) =
    assert add(1, n) == n + 1
```

運行結果顯示為摘要，並且可以以各種文件格式（.md，.csv，etc.）輸出。

## Doc Test

在 Erg 中，，<gtr=“10”/>以後成為註釋行，但在<gtr=“11”/>，<gtr=“12”/>中成為 doc comment，可以通過 VSCode 等編輯器標記註釋。並且，如果 doc comment 中的源代碼被指定為 erg，則通過 erg test 命令進行自動測試。以下是測試的例子。


```erg
VM = ...
    ...
    #[[
    execute commands.
    ```erg
    # VM in standard configuration
    {vm1; ...} = import "tests/mock"

    assert vm1.exec!("i = 0") == None
    assert vm1.exec!("i").try_into(Int)? == 0
    ```
    ]]#.exec! ref self, src =
        ...
    ...
```

測試時使用的模擬對象（嘲笑對象）定義在模塊中。