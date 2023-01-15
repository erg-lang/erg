# erg-linter (WIP)

erg-linter (can be used with `erg lint`) is a tool to check the erg file for errors.

## Features

The following codes are warned.

* Unreachable codes
* Wildcard import
* Unused variables
* Shadowing of built-in variables
* Unused objects that are not `NoneLike`
* Procedures without side-effects
* Variables that can be defined as constants
* Unnecessary `.clone`
* Mutable objects that do not change
* Hardcoded well-known constants (e.g. `3.14`)
* Defining a subroutine with too many parameters
