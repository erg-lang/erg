from _erg_result import Error
from _erg_control import then__

class Complex(complex):
    def try_new(i):
        if isinstance(i, Complex):
            Complex(i)
        else:
            Error("not a complex")

    def mutate(self):
        return ComplexMut(self)

    def __str__(self) -> str:
        return super().__str__()

    def __repr__(self) -> str:
        return super().__repr__()
    
    def __add__(self, other):
        return then__(complex.__add__(self, other), Complex)
    
    def __sub__(self, other):
        return then__(complex.__sub__(self, other), Complex)
    
    def __mul__(self, other) :
        return then__(complex.__mul__(self, other), Complex)

    def __div__(self, other):
        return then__(complex.__div__(self, other), Complex)


class ComplexMut:
    value: Complex

    def __init__(self, c) -> None:
        self.value = Complex(c)

    def __repr__(self):
        return self.value.__repr__()

    def __hash__(self):
        return self.value.__hash__()

    def __deref__(self):
        return self.value

    def __eq__(self, other):
        if isinstance(other, Complex):
            return self.value == other
        else:
            return self.value == other.value

    def __ne__(self, other):
        if isinstance(other, Complex):
            return self.value != other
        else:
            return self.value != other.value

    def __le__(self, other):
        if isinstance(other, Complex):
            return self.value <= other
        else:
            return self.value <= other.value

    def __ge__(self, other):
        if isinstance(other, Complex):
            return self.value >= other
        else:
            return self.value >= other.value

    def __lt__(self, other):
        if isinstance(other, Complex):
            return self.value < other
        else:
            return self.value < other.value

    def __gt__(self, other):
        if isinstance(other, Complex):
            return self.value > other
        else:
            return self.value > other.value

    def __add__(self, other):
        if isinstance(other, Complex):
            return ComplexMut(self.value + other)
        else:
            return ComplexMut(self.value + other.value)

    def __sub__(self, other):
        if isinstance(other, Complex):
            return ComplexMut(self.value - other)
        else:
            return ComplexMut(self.value - other.value)

    def __mul__(self, other):
        if isinstance(other, Complex):
            return ComplexMut(self.value * other)
        else:
            return ComplexMut(self.value * other.value)

    def __floordiv__(self, other):
        if isinstance(other, Complex):
            return ComplexMut(self.value // other)
        else:
            return ComplexMut(self.value // other.value)

    def __pow__(self, other):
        if isinstance(other, Complex):
            return ComplexMut(self.value**other)
        else:
            return ComplexMut(self.value**other.value)

    def __pos__(self):
        return self

    def __neg__(self):
        return ComplexMut(-self.value)
