# Erg parser

## Use `erg_parser` as a Python library

`erg_parser` can be built as a Python library by using pyo3/maturin.

### Example

```python
import erg_parser

module = erg_parser.parse("x = 1")
for chunk in module:
    if isinstance(chunk, erg_parser.expr.Def):
        assert chunk.sig.inspect() == "x"
```

### Debug install (using venv)

```python
python -m venv .venv
source .venv/bin/activate
maturin develop --features pylib_parser
```

### Release install

```python
maturin build -i python --release --features pylib_parser
pip install <output wheel>
```
