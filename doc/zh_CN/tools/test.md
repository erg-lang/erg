# 测试子命令

erg 命令有一个名为 test 的子命令，它支持测试的实现和执行。

## 测试装饰器 (@Test)

Erg 使用 `erg test` 命令测试包中 `tests` 目录或 `*.test.er` 文件中的 `@Test` 子例程。
`tests` 子例程负责黑盒测试（不测试私有函数），`*.test.er` 子例程负责白盒测试（也测试私有函数）。

```python
# tests/test1.er
{add; ...} = import "foo"

@Test
test_1_plus_n(n: Nat) =
    assert add(1, n) == n + 1
```

执行结果以摘要形式显示，可以以各种文件格式（.md、.csv 等）输出。

## 文档测试

在 Erg 中，`#` 和 `#[` 是注释行，但 `##` 和 `#[[` 是 doc 注释，并且注释可以从 VSCode 等编辑器显示为 markdown。
此外，如果指定了 erg，则使用 erg test 命令自动测试文档注释中的源代码。
下面是一个示例测试。

```python
VMs =...
    ...
    #[[
    execute commands.
    ```python
    # 标准配置的虚拟机
    {vm1; ...} = import "tests/mock"

    assert vm1.exec!("i = 0") == None
    assert vm1.exec!("i").try_into(Int)? == 0
    ```
    ]]#
    .exec! ref self, src =
        ...
    ...
```

用于测试的模拟对象（mock objects）在 `tests/mock` 模块中定义。