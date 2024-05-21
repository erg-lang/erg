# Name resolving

The name resolution phase of Erg is currently integrated with the type analysis phase.
This is not considered a good design, and it is planned to be separated in the future.

The tasks performed in the name resolution phase are as follows:

* Associate variable names with scopes, assign unique IDs, and assign type variables if necessary
* Reorder constants according to dependencies
* Evaluate constant expressions and replace them if possible (this may be separated from the name resolution phase)
