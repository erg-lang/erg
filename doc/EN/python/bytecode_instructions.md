# Python Bytecode Instructions

Python bytecode variable manipulation instructions are accessed through namei (name index). This is to realize Python's dynamic variable access (access by string using eval, etc.).
Each instruction is 2 bytes, and instructions and arguments are stored in a little endian.
Instructions that do not take arguments also use 2 bytes (the argument part is 0).

## STORE_NAME(namei)

```python
globals[namei] = stack.pop()
```

## LOAD_NAME(namei)

```python
stack.push(globals[namei])
```

Only called at the top level.

## LOAD_GLOBAL(namei)

```python
stack.push(globals[namei])
```

To Load what was STORE_NAME at the top level in the inner scope, but namei at the top level is not necessarily the same as namei in a code object of a certain scope (namei, not name, is the same).

## LOAD_CONST(namei)

```python
stack.push(consts[namei])
```

Load constants in the constants table.
Currently (Python 3.9), CPython MAKE_FUNCTION every time a lambda function is called "\<lambda\>".

````console
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
````

## STORE_FAST(namei)

```python
fastlocals[namei] = stack.pop()
```

Probably corresponds to STORE_NAME at the top level.
This is supposed to store an unreferenced (or single) variable.
Is it for optimization that the global space has its own instruction?

## LOAD_FAST(namei)

stack.push(fastlocals[namei])
fastlocals is varnames?

## LOAD_CLOSURE(namei)

```python
cell = freevars[namei]
stack.push(cell)
```

After that BUILD_TUPLE is called.
It is only called in a closure, and cellvars is supposed to store references in the closure.
Unlike LOAD_DEREF, the entire cell (container with references) is pushed onto the stack

## STORE_DEREF(namei)

```python
cell = freevars[namei]
cell.set(stack.pop())
```

Variables without references in the inner scope are STORE_FAST, but referenced variables are STORE_DEREF.
In Python, the reference count is increased or decreased within this instruction

## LOAD_DEREF(namei)

```python
cell = freevars[namei]
stack.push(cell.get())
```

## Name List

### varnames

List of names of internal variables of the function corresponding to `fast_locals`.
Even if there is a variable with the same name in names, it is not basically the same (it is newly created, and the outside variable cannot be accessed from its scope).
In other words, variables defined in scope without external references go into varnames

### names

Corresponding to `globals`.
A list of names of external constants (reference only) used in a scope (even ordinary variables at the top level go into names).
In other words, constants defined outside the scope go into names

## free variable

Corresponds to `freevars`.
Variables captured by closure. It behaves static within the same function instance.

## cell variables

Corresponds to `cellvars`.
Variables captured by an inner closure function within a function. A copy is made, so the original variable remains intact.
