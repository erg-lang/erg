# `erg` 构建功能

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/build_features.md%26commit_hash%3Dddb483c2cf733dba776fd6a5589f28871a2c3e62)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/build_features.md&commit_hash=ddb483c2cf733dba776fd6a5589f28871a2c3e62)

## debug

进入调试模式。结果，Erg 内部的行为顺序显示在日志中
独立于 Rust 的 `debug_assertions` 标志

## japanese

将系统语言设置为日语
Erg 内部选项、帮助(帮助、版权、许可证等)和错误显示为日语

## simplified_chinese

将系统语言设置为简体中文
Erg 内部选项、帮助(帮助、版权、许可证等)和错误显示为简体中文

## traditional_chinese

将系统语言设置为繁体中文
Erg 内部选项、帮助(帮助、版权、许可证等)和错误显示为繁体中文。

## unicode/pretty

使得编译器显示丰富内容

## large_thread

增加线程堆栈大小。用于Windows执行和测试执行

## els

通过 `--language-server` 使其变得可用
通过 `erg --language-server` 打开

## py_compatible

启用Python兼容模式，使部分api和语法与Python兼容。用于[pylyzer](https://github.com/mtshiba/pylyzer)
