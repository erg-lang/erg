# Branch naming and operation policy

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/branches.md%26commit_hash%3Dfc7a25a8d86c208fb07beb70ccc19e4722c759d3)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/branches.md&commit_hash=fc7a25a8d86c208fb07beb70ccc19e4722c759d3)

* Basically, development is done on a single `main` branch (monorepo development). Create a `feature-*` or `issue-*` branch only if it is difficult to work without a separate branch.

## main

* main development branch
* The following conditions must be met

* compile successfully

## beta (not created at this time)

* Latest beta release
* The following conditions must be met

* Compile succeeds.
* all tests succeed

## feature-*

* A branch that develops one specific feature.
* main is cut off and created.

* No conditions

## issue-*

* branch that resolves a specific issue

* no condition
