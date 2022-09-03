# test 子命令

erg 指令中有 test 这个子指令，进行测试安装以及执行的支援。

## 测试装饰器（@Test）

Erg 使用命令测试软件包中的<gtr=“3”/>目录或<gtr=“4”/>文件中的<gtr=“5”/>子程序。<gtr=“7”/>子例程负责黑盒测试（不测试私有函数），<gtr=“8”/>子例程负责白盒测试（也测试私有函数）。


```erg
# tests/test1.er
{add; ...} = import "foo"

@Test
test_1_plus_n(n: Nat) =
    assert add(1, n) == n + 1
```

运行结果显示为摘要，并且可以以各种文件格式（.md，.csv，etc.）输出。

## Doc Test

在 Erg 中，，<gtr=“10”/>以后成为注释行，但在<gtr=“11”/>，<gtr=“12”/>中成为 doc comment，可以通过 VSCode 等编辑器标记注释。并且，如果 doc comment 中的源代码被指定为 erg，则通过 erg test 命令进行自动测试。以下是测试的例子。


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

测试时使用的模拟对象（嘲笑对象）定义在模块中。
