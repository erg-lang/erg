# 包管理器

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/pack.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/pack.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Erg 标配有一个包管理器，您可以使用 `pack` 子命令调用它。
以下是典型的选项。

* `erg pack init`：将当前目录初始化为一个包。会生成一个 `package.er` 文件和一个 `src` 目录。指定 `app` 将产生一个可执行包，`lib` 将产生一个库包，而 `hybrid` 将产生两个包。如果指定了 `--license`，将自动放置许可文件。
* `erg pack build`：构建一个包。使用 `--release` 可以运行和优化测试。工件放置在 `build/debug` 或 `build/release` 中。
* `erg pack install`：安装一个包。在库的情况下，`src` 被放置在 `.erg/lib` 中，而应用程序作为 shell 脚本被放置在 `.erg/app` 中。使用 `--release` 进行优化。
* `erg pack run`：构建包并运行应用程序(仅限应用程序包)。
* `erg pack clean`：删除构建目录的内容。
* `erg pack test`：运行包测试。有关详细信息，请参阅 [test.md](./test.md)。
* `erg pack publish`：发布/发布包。您将需要一个 GitHub 帐户和公钥。

本文档解释了如何管理您自己的包。
如果要安装或搜索外部包，请参阅 [install.md](./install.md)。
另请参阅 [package_system.md](../syntax/33_package_system.md) 了解 Erg 包系统。

## 整个包的标准目录结构(对于应用程序包)

```console
/package # package root directory
    /build # Directory to store build results
        /debug # Artifacts during debug build
        /release # Artifacts of release build
    /doc # Documents (in addition, by dividing into subdirectories such as `en`, `ja` etc., it is possible to correspond to each language)
    /src # source code
        /main.er # file that defines the main function
    /tests # Directory to store (black box) test files
    /package.er # file that defines package settings
```

## package.er

`erg pack init` 将生成如下所示的 `package.er` 文件。 `package.er` 描述了包的配置。
下面是一个`package.er`的例子。

```python
name = "example" # package 名称
author = "John Smith" # package 作者名称
version="0.1.0"
description = "An awesome package"
categories = ["cli"] # package 类别
type = "app" # "app" 或者 "lib"
license = "" # 例如"MIT", "APACHE-2.0", "MIT OR Apache-2.0"
pre_build = "" # 构建前要执行的脚本文件名
post_build = "" # 构建后要执行的脚本文件名
dependencies = {
    # 如果不指定版本，则选择最新的
    # 如果省略版本说明，包管理器会自动将上次成功构建的版本添加到注释中
    foo = pack("foo") # [INFO] 最后成功构建的版本：1.2.1
    # 包可以重命名
    bar1 = pack("bar", "1.*.*") # [INFO] 最后成功构建的版本：1.2.0
    bar2 = pack("bar", "2.*.*") # [INFO] 最后成功构建的版本：2.0.0
    baz = pack("baz", "1.1.0")
}
deprecated=False
successors = [] # 替代包(当一个包被弃用时)
```

## 语义版本控制

Erg 包是基于 [语义版本控制](https://semver.org/lang/en/) 进行版本控制的。
语义版本控制大致以"x.y.z"格式指定(x、y、z 是大于或等于 0 的整数)。
每个数字的含义如下。

* x：主要版本(更新破坏兼容性时增加 1)
* y：次要版本(执行兼容更新时增加1(API添加，弃用等)，错误修复等由补丁版本升级处理)
* z：补丁版本(当进行小的更改以修复错误或保持兼容性时增加1，破坏兼容性的严重修复由主要版本升级处理)

但是，默认情况下，版本 `0.*.*` 中的更改始终是不兼容的。如果要在保持兼容性的同时升级，请在其后指定 `-compatible`(Erg 自己的规则)。例如，如果要在保持与 0.2.1 兼容的同时添加功能，即要升级到 0.3.0，则指定 0.3.0-compatible。如果您已修复错误，还请指定"0.2.2-compatible"。
该版本将被视为与以前的版本兼容。
即使您想将 `0.*.*` 升级到 `1.0.0`，这仍然有效。也就是说，`1.0.0-compatible` 与之前的版本 `0.y.z` 兼容。

生成锁文件时，语义版本控制非常重要。锁定文件是为保持依赖项兼容而生成的文件，因此除非明确更新，否则较新版本的依赖项依赖于较旧的包。
当多人开发具有依赖包的包时，锁定文件很有用。它还通过允许依赖于它们的包在兼容的情况下重用包来节省本地存储。

Erg 的包管理器严格执行这些规则，并将拒绝违反这些规则的包更新。
Erg 包管理器与版本控制系统(例如 git)一起使用，以检测代码差异并在发布包时验证版本控制的正确性。
具体来说，包管理器会查看 API 的类型。如果类型是旧版本的子类型，则认为更改是兼容的(请注意，这不是完整的验证；类型兼容但语义上不兼容的重大更改是可能的，这是开发人员的工作来确定这一点)。

此外，由于整个包存储库都在注册表中注册，即使是开发人员也无法在不通过包管理器的情况下更新包。
此外，包可以被弃用但不能被删除。

### 附录：语义版本控制问题和对策

语义版本控制存在(至少)两个已知问题。
首先，语义版本控制可能过于严格。
使用语义版本控制，单个不兼容的 API 更改会增加整个包的主要版本。
发生这种情况时，诸如"我想尝试一个新的 API，但我必须处理另一个不兼容的 API 更改，所以我不会升级"之类的事情。
其次，语义版本控制可以承诺太多。
如上一节所述，对 API 的"兼容更改"在理论上是不可证明的。如果您指定要使用版本为 `1.0.1` 的包，则可以在语义版本控制方面使用 `1.0.1` 和 `2.0.0` 之间的任何包(`1.0.0` 不能被使用，因为错误已被修复)，但由于包开发人员无意使用 API，构建可能不会成功。

Erg 通过允许同时使用不同版本的包(通过重命名)解决了这个问题。这使得在部分引入 ver2 API 的同时继续使用 ver1 API 成为可能。
此外，虽然这不是一个非常理想的状态，但如果只能使用 API 的某个次要版本而没有错误，则可以不理会它并继续前进到下一个版本。

## 发布

可以使用 `publish` 子命令发布包。发布需要 GitHub 帐户。
默认情况下，包使用 `(owner_name)/(package_name)` 注册。如果满足一定条件(下载次数、维护频率等)，可以申请注册一个省略所有者名称的别名。
请注意，包名称不区分大小写，并且不区分诸如 `_` 和 `-` 之类的分隔符。