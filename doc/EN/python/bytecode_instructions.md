# Python Bytecode Instructions

Python bytecode variable manipulation commands are accessed through namei (name index). This is to achieve dynamic variable access in Python (which can be accessed as a string using eval, etc.).
One instruction is 2 bytes, and the instruction and arguments are stored in little endian.
Instructions that do not take arguments also use 2 bytes (the argument part is 0).

* Change in 3.11: Instructions are no longer fixed length and some instructions exceed 2 bytes. The extra byte sequence is zero in most cases, and its purpose is unknown, but it is thought to be an optimization option. The known irregular byte length instructions are as follows.
  * `PRECALL` (4 bytes)
  * `CALL` (10 byte)
  * `BINARY_OP` (4 byte)
  * `STORE_ATTR` (10 byte)
  * `COMPARE_OP` (6 byte)
  * `LOAD_GLOBAL` (12 byte)
  * `LOAD_ATTR` (10 byte)

## STORE_NAME(namei)

```python
globals[namei] = stack.pop()
```

## LOAD_NAME(namei)

```python
stack.push(globals[namei])
```

Only called at top level.

## LOAD_GLOBAL(namei)

```python
stack.push(globals[namei])
```

It is for loading STORE_NAME at the top level in the inner scope, but `namei` at the top level is not necessarily the same as namei in the code object of a certain scope (name is the same, not namei)

## LOAD_CONST(namei)

```python
stack.push(consts[namei])
```

Load constants in the constant table.
Currently (Python 3.9), in CPython, each lambda function is MAKE_FUNCTION with the name "\<lambda\>"

```console
>>> dis.dis("[1,2,3].map(lambda x: x+1)")
1 0 LOAD_CONST 0 (1)
        2 LOAD_CONST 1 (2)
        4 LOAD_CONST 2 (3)
        6 BUILD_LIST 3
        8 LOAD_ATTR 0 (map)
        10 LOAD_CONST 3 (<code object <lambda> at 0x7f272897fc90, file "<dis>", line 1>)
        12 LOAD_CONST 4 ('<lambda>')
        14 MAKE_FUNCTION 0
        16 CALL_FUNCTION 1
        18 RETURN_VALUE
```

## STORE_FAST(namei)

fastlocals[namei] = stack.pop()
Possibly corresponds to STORE_NAME at top level
The unreferenced (or single) variable is assumed to be stored by this
Is it for optimization that the global space has its own instructions?

## LOAD_FAST(namei)

stack.push(fastlocals[namei])
fastlocals are varnames?

## LOAD_CLOSURE(namei)

```python
cell = freevars[namei]
stack. push(cell)
```

Then BUILD_TUPLE is called
It is only called inside the closure, and cellvars are supposed to store references inside the closure.
Unlike LOAD_DEREF, each cell (container filled with references) is pushed to the stack

## STORE_DEREF(namei)

```python
cell = freevars[namei]
cell.set(stack.pop())
```

Variables without references in inner scopes are STORE_FAST, but referenced variables are STORE_DEREF
In Python, the reference count is incremented and decremented within this instruction

## LOAD_DEREF(namei)

```python
cell = freevars[namei]
stack.push(cell.get())
```

## name list

### varnames

Name list of function internal variables corresponding to fast_locals
Even if there are variables with the same name in names, they are basically not the same (newly created and outside variables cannot be accessed from that scope)
i.e. variables without external references defined in scope go into varnames

### names

Compatible with globals
Name list of external constants (only referenced) used within the scope (at the top level, even ordinary variables are included in names)
i.e. constants defined outside the scope go into names

## free variables

Compatible with freevars
Variables captured by the closure. It behaves statically within the same function instance.

## cell variables

Corresponds to cellvars
Variables captured within a function to an inner closure function. Since a copy is made, the original variable remains as it is.