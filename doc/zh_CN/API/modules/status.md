 # 模块`status`

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/modules/status.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/modules/status.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

定义了一个类型来表示状态。请根据情况删除选项来使用它

* ExecResult = {"success", "warning", "failure", "fatal", "unknown"}
* ExecStatus = {"ready", "running", "sleeping", "plague", "completed", "terminated"}