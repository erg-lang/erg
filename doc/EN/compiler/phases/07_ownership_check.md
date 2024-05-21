# Ownership checking

Ownership checking is performed by a structure called `OwnershipChecker`.
The ownership checking performs the following tasks:

* Check if mutable objects are not referenced after being moved
