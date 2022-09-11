# Python Bytecode Instructions

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/python/bytecode_instructions.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/python/bytecode_instructions.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

Python bytecodeの変数操作系の命令はnamei (name index)を通してアクセスされる。これは、Pythonの動的変数アクセス(evalなどを使い、文字列でアクセスできる)を実現するためである。
1命令は2byteで、命令、引数がlittle endianで格納されている。
引数を取らない命令も2byte使っている(引数部は0)。

## STORE_NAME(namei)

```python
globals[namei] = stack.pop()
```

## LOAD_NAME(namei)

```python
stack.push(globals[namei])
```

トップレベルでしか呼び出されない。

## LOAD_GLOBAL(namei)

```python
stack.push(globals[namei])
```

トップレベルでSTORE_NAMEしたものを内側のスコープでLoadするためのものだが、トップレベルでの`namei`ならばあるスコープのコードオブジェクトでのnameiとも同じとは限らない(nameiではなくnameが同じ)

## LOAD_CONST(namei)

```python
stack.push(consts[namei])
```

定数テーブルにある定数をロードする。
現在(Python 3.9)のところ、CPythonではいちいちラムダ関数を"\<lambda\>"という名前でMAKE_FUNCTIONしている

```console
>>> dis.dis("[1,2,3].map(lambda x: x+1)")
1       0 LOAD_CONST               0 (1)
        2 LOAD_CONST               1 (2)
        4 LOAD_CONST               2 (3)
        6 BUILD_LIST               3
        8 LOAD_ATTR                0 (map)
        10 LOAD_CONST               3 (<code object <lambda> at 0x7f272897fc90, file "<dis>", line 1>)
        12 LOAD_CONST               4 ('<lambda>')
        14 MAKE_FUNCTION            0
        16 CALL_FUNCTION            1
        18 RETURN_VALUE
```

## STORE_FAST(namei)

fastlocals[namei] = stack.pop()
おそらくトップレベルにおけるSTORE_NAMEに対応する
参照のない(もしくは単一)変数がこれによって格納されると思われる
わざわざグローバル空間が独自の命令を持っているのは最適化のため?

## LOAD_FAST(namei)

stack.push(fastlocals[namei])
fastlocalsはvarnames?

## LOAD_CLOSURE(namei)

```python
cell = freevars[namei]
stack.push(cell)
```

そのあとBUILD_TUPLEが呼ばれている
クロージャの中でしか呼び出されないし、cellvarsはクロージャの中での参照を格納するものと思われる
LOAD_DEREFと違ってcell(参照を詰めたコンテナ)ごとスタックにpushする

## STORE_DEREF(namei)

```python
cell = freevars[namei]
cell.set(stack.pop())
```

内側のスコープで参照のない変数はSTORE_FASTされるが、参照される変数はSTORE_DEREFされる
Pythonではこの命令内で参照カウントの増減がされる

## LOAD_DEREF(namei)

```python
cell = freevars[namei]
stack.push(cell.get())
```

## 名前リスト

### varnames

fast_localsに対応する、関数の内部変数の名前リスト
namesで同名の変数があっても、基本的に同じものではない(新しく作られ、そのスコープからは外の変数にアクセスできない)
つまり、スコープ内で定義された外部参照のない変数はvarnamesに入る

### names

globalsに対応
スコープ内で使われた外部定数(参照だけしている)の名前リスト(トップレベルでは普通の変数でもnamesに入る)
つまり、スコープ外で定義された定数はnamesに入る

## free variable

freevarsに対応
クロージャがキャプチャした変数。同じ関数インスタンス内においてstaticな振る舞いをする。

## cell variables

cellvarsに対応
関数内で内側のクロージャ関数にキャプチャされる変数。コピーが作られるので、元の変数はそのまま。
