# 环境子命令

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/env.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/env.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

env 子命令指定 erg 执行环境。
使用 `erg env new [env name]` 创建一个新的执行环境。 将打开一个交互式工具，当您指定 erg 版本时，将安装该版本的 erg(如果已存在，将使用它)，您将能够将其用作新环境。
您可以使用 `erg env switch [env name]` 切换环境。
可以使用 `erg env edit` 编辑创建的环境以预安装软件包并指定其他语言的依赖项。
该命令最大的特点是`erg env export`可以将重现环境的信息输出为`[env name].env.er`文件。 这使您可以立即开始在与其他人相同的环境中进行开发。 此外，`erg env publish` 可以像包一样发布环境。