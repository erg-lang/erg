# 高級中間表示(HIR, High-level Intermediate Representation)

HIR 是 Erg 編譯器從 AST 生成的結構
此結構包含源代碼中每個表達式的完整類型信息，并且在語法上已脫糖
AST與源代碼一一對應(純文本)，但是HIR去掉了不必要的代碼信息，添加了省略的類型信息，所以HIR可以轉換為源代碼很難恢復
讓我們在下面的代碼中查看 HIR 的示例

```python
v = ![]
for! 0..10, i =>
    v.push! i
log v.sum()
```

從此代碼生成的 AST 如下所示：

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

從 AST 生成的 HIR 如下所示：

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
            t: ...Object => NoneType,
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

對象類型被推斷為盡可能小。 另一方面，子例程推斷實現存在的類型
因此，實際參數的類型和形式參數的類型可能不匹配