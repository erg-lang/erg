# Sealed

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Sealed.md%26commit_hash%3D79152ee1dfdc6c7a76d68c608b363d5d3c1a0031)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Sealed.md&commit_hash=79152ee1dfdc6c7a76d68c608b363d5d3c1a0031)

Classes and traits can be sealed. When a class is sealed, it can't be inherited from external modules, and when a trait is sealed, it can't be implemented from external modules. However, in both cases, they can still be referenced from external modules.

