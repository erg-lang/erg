# Name resolving

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/04_name_resolve.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/04_name_resolve.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

The name resolution phase of Erg is currently integrated with the type analysis phase.
This is not considered a good design, and it is planned to be separated in the future.

The tasks performed in the name resolution phase are as follows:

* Associate variable names with scopes, assign unique IDs, and assign type variables if necessary
* Reorder constants according to dependencies
* Evaluate constant expressions and replace them if possible (this may be separated from the name resolution phase)
