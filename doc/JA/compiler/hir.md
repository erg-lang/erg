# 高レベル中間表現(HIR, High-level Intermediate Representation)

HIRはErgコンパイラがASTから生成する構造体です。
この構造体にはソースコード中のあらゆる式の完全な型情報が含まれており、また構文糖が脱糖されています。
ASTは(プレーンテキストとしての)ソースコードと一対一対応しますが、HIRは不要なコードの情報が除去されていたり、また省略された型情報が付記されたりしているため、HIRからソースコードを復元することは困難です。
以下のコードでHIRの例を見てみましょう。

```erg
v = ![]
for! 0..10, i =>
    v.push! i
log v.sum()
```

このコードから生成されるASTは以下のようになります。

```erg
AST(Module[
    VarDef{
        sig: VarSignature{
            pat: VarPattern::Name(Name("v")),
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
                            pat: ParamPattern::Name(Name("i")),
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

そしてASTから生成されるHIRは以下のようになります。

```erg
HIR(Module[
    VarDef{
        sig: VarSignature{
            pat: VarPattern::Name(Name("v")),
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

オブジェクトの型は可能な限り小さく推論されます。それに対し、サブルーチンは実装が存在する型が推論されます。
なので、実引数の型と仮引数の型が合わない場合もあります。
