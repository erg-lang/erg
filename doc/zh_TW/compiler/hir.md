# 高級中間表示(HIR, High-level Intermediate Representation)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/hir.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/hir.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

HIR 是 Erg 編譯器從 AST 生成的結構。
此結構包含源代碼中每個表達式的完整類型信息，并且在語法上已脫糖。
AST 與源代碼（作為純文本）具有一一對應的關系，但是 HIR 去除了不必要的代碼信息并添加了省略的類型信息，因此將 HIR 轉換回源代碼是困難的。讓我們看下面代碼中的 HIR 示例。

```python
v = ![]
for! 0..10, i =>
    v.push! i
log v.sum()
```

從此代碼生成的 AST 如下所示: 

```python
AST(Module[
    VarDef{
        sig: VarSignature{
            pat: VarPattern::Ident(None, VarName("v")),
            spec_t: None,
        },
        op: "=",
        body: Block[
            UnaryOp{
                op: "!",
                expr: Array([]),
            },
        ],
    },
    Call{
        obj: Accessor::Local("for!"),
        args: [
            BinOp{
                op: "..",
                lhs: Literal(0),
                rhs: Literal(10),
            },
            Lambda{
                sig: LambdaSignature{
                    params: [
                        ParamSignature{
                            pat: ParamPattern::Name(VarName("i")),
                        },
                    ],
                    spec_ret_t: None,
                },
                body: Block[
                    Call{
                        obj: Accessor::Attr{"v", "push!"},
                        args: [
                            Accessor::Local("i"),
                        ],
                    },
                ],
            },
        ],
    },
    Call{
        obj: Accessor::Local("log"),
        args: [
            Call{
                obj: Accessor::Attr("v", "sum"),
                args: [],
            }
        ],
    }
])
```

從 AST 生成的 HIR 如下所示: 

```python
HIR(Module[
    VarDef{
        sig: VarSignature{
            pat: VarPattern::Ident(None, Name("v")),
            t: [0..10, _]!,
        },
        op: "=",
        body: Block[
            expr: UnaryOp{
                op: "!",
                expr: Array([]),
                t: [0..10, 0]!,
            },
        ],
    },
    Call{
        obj: Accessor::Local{
            name: "for!",
            t: (Range Nat, Nat => NoneType) => NoneType,
        },
        args: [
            BinOp{
                op: "..",
                lhs: Literal(0),
                rhs: Literal(10),
                t: 0..10,
            },
            Lambda{
                sig: LambdaSignature{
                    params: [
                        ParamSignature{
                            pat: ParamPattern::Name(Name("i")),
                            t: 0..10,
                        },
                    ],
                    t: 0..10 => NoneType,
                },
                body: Block[
                    Call{
                        obj: Accessor::Attr{
                            obj: Accessor::Local("v"),
                            field: "push!",
                            t: Ref!(Self![T ~> T, N ~> N+1]).(Nat) => NoneType,
                        },
                        args: [
                            Accessor::Local("i"),
                        ],
                    },
                ],
            },
        ],
    },
    Call{
        obj: Accessor::Local{
            name: "log",
            *t: Obj => NoneType,
        },
        args: [
            Call{
                obj: Accessor::Attr{
                    obj: Accessor::Local("v"),
                    field: "sum",
                    t: [0..10, !_] -> Nat
                },
                args: [],
                t: Nat
            }
        ],
    }
])
```

對象類型推斷盡可能地準確。另一方面，子程序會推斷存在實現的類型。因此，實際參數的類型和形式參數的類型可能不匹配。
