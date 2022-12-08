from _erg_result import Error
from _erg_int import Int
from _erg_int import IntMut

class Nat(Int):
    def try_new(i): # -> Result[Nat]
        if i >= 0:
            return Nat(i)
        else:
            return Error("Nat can't be negative")

    def times(self, f):
        for _ in range(self):
            f()

    def saturating_sub(self, other):
        if self > other:
            return self - other
        else:
            return 0
    def mutate(self):
        return NatMut(self)

class NatMut(IntMut): # and Nat
    value: Nat

    def __init__(self, n):
        self.value = n
    def __repr__(self):
        return self.value.__repr__()
    def __eq__(self, other):
        if isinstance(other, Int):
            return self.value == other
        else:
            return self.value == other.value
    def __ne__(self, other):
        if isinstance(other, Int):
            return self.value != other
        else:
            return self.value != other.value
    def __le__(self, other):
        if isinstance(other, Int):
            return self.value <= other
        else:
            return self.value <= other.value
    def __ge__(self, other):
        if isinstance(other, Int):
            return self.value >= other
        else:
            return self.value >= other.value
    def __lt__(self, other):
        if isinstance(other, Int):
            return self.value < other
        else:
            return self.value < other.value
    def __gt__(self, other):
        if isinstance(other, Int):
            return self.value > other
        else:
            return self.value > other.value
    def __add__(self, other):
        if isinstance(other, Nat):
            return NatMut(self.value + other)
        else:
            return NatMut(self.value + other.value)
    def __mul__(self, other):
        if isinstance(other, Nat):
            return NatMut(self.value * other)
        else:
            return NatMut(self.value * other.value)
    def __pow__(self, other):
        if isinstance(other, Nat):
            return NatMut(self.value ** other)
        else:
            return NatMut(self.value ** other.value)
    def try_new(i): # -> Result[Nat]
        if i >= 0:
            return NatMut(i)
        else:
            return Error("Nat can't be negative")

    def times(self, f):
        for _ in range(self.value):
            f()
