# Tensor Shape: [Nat; N]

   A class for efficiently manipulating multidimensional arrays. It also defines operations such as multiplication on multidimensional arrays.
   Matrix, Vector, etc. inherit from this type.

``` erg
Tensor.arrange(0..9) #Tensor[10]
```

* reshape(self, NewShape: [Nat; M]) -> Self NewShape

``` erg
(1..9).into(Tensor).reshape[3, 3]
```

* identity i: Nat -> Self shape: [Nat; N]
* zeros(Shape: [Nat; N]) -> Self
* ones(Shape: [Nat; N]) -> Self

* diag

* linspace
*logspace
* geomspace