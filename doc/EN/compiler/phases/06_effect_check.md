# Side-effect checking

Side-effect checking is performed by a structure called EffectChecker.
The side-effect checking performs the following tasks:

* Check if a call with side effects is made within an expression where side effects are not allowed.
* Check if mutable objects are referenced within a function.
* Check if a procedure is assigned to a function.
