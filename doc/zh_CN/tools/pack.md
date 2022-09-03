# 包管理器

Erg 标配包管理器，可通过子命令调用。以下是典型的选项。

* ：将当前目录初始化为软件包。生成文件和<gtr=“6”/>目录。指定<gtr=“7”/>是可执行文件的软件包，指定<gtr=“8”/>是库的软件包，指定<gtr=“9”/>是两个软件包。如果指定<gtr=“10”/>，它将自动放置许可证文件。
* ：构建包。执行测试并进行优化。工件配置在或<gtr=“14”/>。
* ：安装软件包。对于库，被放置在<gtr=“17”/>以下，应用程序被放置在<gtr=“18”/>作为 shell 脚本。添加时进行最优化。
* ：构建软件包并运行应用程序（仅限 app 软件包）。
* 删除：build 目录中的内容。
* ：测试软件包。有关详细信息，请参见。
* ：发布/发布软件包。我需要 GitHub 帐户和公钥。

另外，本文档说明了管理自己的软件包时的方法。要安装或搜索外部软件包，请参阅。此外，有关 Erg 的封装系统，请参见<gtr=“26”/>。

## 整个软件包的标准目录配置（适用于应用程序软件包）


```console
/package # パッケージのルートディレクトリ
    /build # ビルド結果を格納するディレクトリ
        /debug # デバッグビルド時の成果物
        /release # リリースビルド時の成果物
    /doc # ドキュメント(さらに`en`, `ja`などのサブディレクトリに分けることで各国語対応可能)
    /src # ソースコード
        /main.er # main関数を定義するファイル
    /tests # (ブラックボックス)テストファイルを格納するディレクトリ
    /package.er # パッケージの設定を定義するファイル
```

## package.er

如果，就会生成以下文件<gtr=“28”/>。<gtr=“29”/>中记述了软件包的设定。以下是<gtr=“30”/>的记述例子。


```erg
name = "example" # package name
author = "John Smith" # package author name
version = "0.1.0"
description = "An awesome package"
categories = ["cli"] # package categories
type = "app" # "app" or "lib"
license = "" # e.g. "MIT", "APACHE-2.0", "MIT OR Apache-2.0"
pre_build = "" # script filename to be executed before build
post_build = "" # script filename to be executed after build
dependencies = {
    # The latest one is selected if the version is not specified
    # If the version specification is omitted, the package manager automatically adds the version of the last successful build to the comments
    foo  = pack("foo") # [INFO] the last successfully built version: 1.2.1
    # Packages can be renamed
    bar1 = pack("bar", "1.*.*") # [INFO] the last successfully built version: 1.2.0
    bar2 = pack("bar", "2.*.*") # [INFO] the last successfully built version: 2.0.0
    baz  = pack("baz", "1.1.0")
}
deprecated = False
successors = [] # alternative packages (when a package is deprecated)
```

## 语义

Erg 软件包根据指定版本。语义版本化通常以<gtr=“31”/>格式指定，其中 x，y 和 z 是大于或等于 0 的整数。每个数字的含义如下。

* x：主要版本（增加一个，以进行破坏兼容性的更新）
* y：次要版本（兼容更新（如 API 添加或过时）时增加 1；补丁版本升级（如错误修复）
* z：修补程序版本（错误修复和兼容性较小的更改增加 1；严重的不兼容修复将在主要版本升级中提供）

但是，在默认情况下，对版本的更改始终不兼容。如果你想在版本升级时保持兼容性，请在后面指定<gtr=“34”/>（Erg 自己的规则）。例如，如果你想添加<gtr=“35”/>，同时兼容，即升级到<gtr=“36”/>，请指定<gtr=“37”/>。如果已修复错误，请指定<gtr=“38”/>。这将确保该版本与上一版本兼容。如果你想将<gtr=“39”/>升级为<gtr=“40”/>，也可以使用此选项。也就是说，<gtr=“41”/>与上一个版本<gtr=“42”/>兼容。

语义版本化在生成锁定文件时非常重要。锁定文件是为保持相关软件包的兼容性而生成的文件，它依赖于旧软件包，除非明确更新相关软件包的新版本。当多人开发具有相关软件包的软件包时，锁定文件非常有用。它还可以节省本地存储，因为相关软件包可以在兼容的情况下使用更多相关软件包。

Erg 软件包管理器严格执行以上规则，任何与规则相冲突的软件包更新都将被拒绝。Erg 包管理器与版本控制系统（如 git）配合使用，在 publish 包时检测代码差异，以验证版本化的有效性。具体地说，包管理器看 API 的类型。如果类型是旧版本的子类型，则将更改视为兼容（请注意，这不是完全验证。可能存在类型上兼容但语义上不兼容的更改。这是开发人员的工作。

此外，由于软件包在注册表中注册了整个存储库，因此开发人员不能在不通过软件包管理器的情况下更新软件包。此外，软件包可以过时，但不能删除。

### Appendix：语义版本化问题及其对策

语义版本化中存在（至少）两个已知问题。首先，语义版本化可能会施加过大的限制。在语义版本化中，只有一个不兼容的 API 更改会提升整个包的主要版本。这会导致“我想尝试一个新的 API，但因为我必须处理另一个不兼容的 API 更改而推迟升级”。还有一点，语义版本可能承诺过高。如上一节所述，API 的“兼容更改”在理论上无法证明。如果你指定要版本的软件包，则从语义版本的角度来看，可以使用大于或等于<gtr=“44”/>小于<gtr=“45”/>的所有软件包（不能使用<gtr=“46”/>）。但是，实际上，软件包开发人员无意中使用 API 可能会导致构建不成功。

为了解决这个问题，Erg 采取了一种方法，允许你同时使用不同版本的软件包（通过重命名）。这允许你在引入部分版本 2 的 API 的同时继续使用版本 1 的 API。此外，虽然不是很理想，但如果只有某些次要版本的 API 可以在没有错误的情况下使用，则可以将其保留到下一个版本。

## publish

可以使用子命令发布软件包。我需要一个 GitHub 账户来发表。缺省情况下，软件包注册为<gtr=“48”/>。如果满足一定的条件（下载数量，维护频率等），则可以申请注册省略所有者名称的别名。软件包名称不区分大小写和分隔符，如<gtr=“49”/>和<gtr=“50”/>。
