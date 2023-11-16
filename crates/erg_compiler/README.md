# The Erg compiler (codename: Centimetre)

The overall structure is described in detail in [architecture.md(English)](../../doc/EN/compiler/architecture.md).For other language translations of architecture.md, please check them out by yourself.

## Use `erg_compiler` as a Python library

`erg_compiler` can be built as a Python library by using pyo3/maturin.

### Example

```python
import erg_compiler

module = erg_compiler.exec_module(".i = 1")
# foo.er:
# .bar = 1
foo = erg_compiler.__import__("foo")
assert module.i == 1
assert foo.bar == 1
```

```python
import erg_compiler
erg_parser = erg_compiler.erg_parser
erg_ast = erg_compiler.erg_parser.ast

module = erg_parser.parse(".i = 1")
d = module.pop()
d.sig = erg_ast.VarSignature.new(erg_ast.Identifier.public("j"), None)
module.push(d)
ast = erg_ast.AST.new("test", module)
code = erg_compiler.compile_ast(ast)
exec(code)
assert j == 1
```

### Debug install (using venv)

```python
python -m venv .venv
source .venv/bin/activate
maturin develop --features pylib_compiler
```

### Release install

```python
maturin build -i python --release --features pylib_compiler
pip install <output wheel>
```
