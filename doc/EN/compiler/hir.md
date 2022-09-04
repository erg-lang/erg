# High-level Intermediate Representation (HIR)

A HIR is a struct that the Erg compiler generates from an AST.
This struct contains the complete type information for every expression in the source code and is desugared syntactically.
AST has a one-to-one correspondence with the source code (as plain text), but HIR has unnecessary code information removed and omitted type information added, so HIR can be converted to source code is difficult to restore.
Let's see an example of HIR in the code below.

```python
v = ![]
for! 0..10, i =>
    v.push!i
log v.sum()
```

The AST generated from this code looks like this:

```python
AST(Module[
    VarDef{
        sig: VarSignature {
            pat: VarPattern::Ident(None, VarName("v")),
            spec_t: None,
        },
        op: "=",
        body: Block[
            Unary Op {
                op: "!",
                expr: Array([]),
            },
        ],
    },
    Call {
        obj: Accessor::Local("for!"),
        args: [
            BinOp{
                op: "..",
                lhs: Literal(0),
                rhs: Literal(10),
            },
            Lambda{
                sig: LambdaSignature {
                    params: [
                        Param Signature {
                            pat: ParamPattern::Name(VarName("i")),
                        },
                    ],
                    spec_ret_t: None,
                },
                body: Block[
                    Call {
                        obj: Accessor::Attr{"v", "push!"},
                        args: [
                            Accessor::Local("i"),
                        ],
                    },
                ],
            },
        ],
    },
    Call {
        obj: Accessor::Local("log"),
        args: [
            Call {
                obj: Accessor::Attr("v", "sum"),
                args: [],
            }
        ],
    }
])
```

And the HIR generated from the AST looks like this:

```python
HIR(Module[
    VarDef{
        sig: VarSignature {
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
    Call {
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
                sig: LambdaSignature {
                    params: [
                        Param Signature {
                            pat: ParamPattern::Name(Name("i")),
                            t: 0..10,
                        },
                    ],
                    t: 0..10 => NoneType,
                },
                body: Block[
                    Call {
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
    Call {
        obj: Accessor::Local{
            name: "log",
            t: ...Object => NoneType,
        },
        args: [
            Call {
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

Object types are inferred as small as possible. Subroutines, on the other hand, infer the type for which the implementation exists.
Therefore, the type of the actual argument and the type of the formal argument may not match.