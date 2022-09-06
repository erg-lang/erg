# Into T

A type that indicates that it can be type-converted to type T.
Even if there is no inheritance relationship between Self and T, it is defined when the relationship is convertible to each other.
Unlike inheritance, there is no implicit conversion. You must always call the `.into` method.

## methods

* into(self, T) -> T

   do the conversion.