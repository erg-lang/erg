# Contributing to Erg

English | <a href='./doc/CONTRIBUTING/CONTRIBUTING_JA.md'>日本語</a> | <a href='./doc/CONTRIBUTING/CONTRIBUTING_zh-CN.md'>简体中文</a> | <a href='./doc/CONTRIBUTING/CONTRIBUTING_zh-TW.md'>繁體中文</a>

Beginners should read the instructions [here](https://github.com/erg-lang/erg/issues/31#issuecomment-1217505198).

## Documents

If you are thinking of contributing to Erg, you should read documents under [doc/dev_guide](./doc/EN/dev_guide/).
Or you are interested in the internal structure of Erg, [doc/compiler](/doc/JA/compiler/) may provide useful information (currently only Japanese).

## Bug reports

If you find any behavior that you think is a bug in Erg, I would appreciate it if you would [report](https://github.com/erg-lang/erg/issues/new/choose) it. Please make sure that the same bug has not already been reported as an issue.

If you type `cargo run --features debug`, Erg will be built in debug mode. This mode may dump information that may be useful for investigating bugs. I would appreciate it if you could report error logs in this mode.

Also, the environment in which the bug occurred need not be reported if it is clear that the bug was not caused by the environment.

## Document Translation

We are always looking for people to translate our documents into various language versions.

We also welcome people who find that the documentation is outdated compared to other languages and would like to update the content (see [here](https://github.com/erg-lang/erg/issues/48#issuecomment-1218247362) for how to do this).

## Asking questions

If you have any questions, please feel free to ask them on the [Discord channel](https://discord.gg/zfAAUbgGr4).

## Development

Requests are always welcome, but please keep in mind that they will not always be accepted. Many issues have trade-offs.

Don't intercept issues that others have been assigned (Check assignees on GitHub). If it is considered too difficult for one person to handle it, we will call for more support.

Before proposing a new feature, consider whether that feature could be easily solved by combining existing features.

Please write code in a style that is standardized by the Erg team and languages.

## [Code of conduct](./CODE_OF_CONDUCT.md)
