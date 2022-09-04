# Improvements from Python

## Perform static analysis (static type checking, variable and property checking)

The benefit of static type checking cannot be emphasized enough now, but checking for the existence of variables and properties is also a part that comes into play quite a bit.

## Strict scope handling

In Python, statements do not have scopes.
Therefore, variables defined in a `for` or `if` have outside effects. You cannot name variables casually.

```python
for i in range(10):
    x = 1
    print(i + x)
print(x) # 1
```

In Erg, all blocks have scope and are completely isolated.

## Clear distinction between mutable and immutable objects

Python is not clear on the distinction between mutable and immutable / heap and value objects, so you have to keep in mind that tuples are immutable but lists are mutable... You need to keep in mind that tuples are immutable, but lists are mutable... and so on.
Also, if you want to make your own classes immutable, you have to go through a tedious process.

```python
# Can you believe this code is valid for the past versions of Python?
i = 256
assert i is 256
i = 257
assert i is not 257
```

## Traits

Just like Java's interface, you can do contract-based programming.

Python also has ABC (Abstract Base Class), but this kind of structure works best with static typing.

## Resolve dependencies statically

This prevents the annoying experience of running a program for a long time and then running it with an error due to missing modules.

## Built-in package manager

Reproducible builds with a standardized directory structure and build files.
Lock file generation and version control are of course provided.
There is no need to choice or mix anaconda, pyenv, poetry, etc. for each project.
