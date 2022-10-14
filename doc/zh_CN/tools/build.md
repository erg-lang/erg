# 构建子命令

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/build.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/build.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

build 子命令构建包
默认构建中执行的步骤如下: 

1. 检查注释/文档中的代码(doc 下的 md 文件)
2. 编译打包所需的代码
3. 对于应用程序包，生成批处理文件或相当于命令的shell脚本
4. 运行测试

构建完成后的交付物输出到以下目录

* 在调试构建期间: build/debug
* 对于发布构建: build/release