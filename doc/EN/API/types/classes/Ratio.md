# Ratio

A type that represents a rational number. It is mainly used when you want to use fractions.
In fact, the / operator in Erg returns Ratio. 1/3 etc. is not evaluated as 0.33333... and is processed as 1/3. Also, 0.1 is equivalent to 1/10. So `0.1 + 0.2 == 0.3`. It sounds obvious, but in Python it is False.
However, the Ratio type tends to be slightly less efficient than the Float type. Float type should be used at the point where execution speed is important and the exact numerical value is not required. However, as Rob Pike says, premature optimization is the root of all evil. Do a real performance test before discarding the Ratio type and using the Float type. Amateurs unconditionally prefer lighter molds.