from _erg_result import Error
from _erg_int import Int
from _erg_int import IntMut  # don't unify with the above line
from _erg_control import then__


class Nat(Int):
    def try_new(i):  # -> Result[Nat]
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

    def __add__(self, other):
        return then__(super().__add__(other), Nat)

    def __mul__(self, other):
        return then__(super().__mul__(other), Nat)

    def __pos__(self):
        return self


class NatMut(IntMut):  # and Nat
    value: Nat

    def __init__(self, n: Nat):
        self.value = n

    def __int__(self):
        return self.value.__int__()

    def __repr__(self):
        return self.value.__repr__()

    def __hash__(self):
        return self.value.__hash__()

    def __eq__(self, other):
        if isinstance(other, int):
            return self.value == other
        else:
            return self.value == other.value

    def __ne__(self, other):
        if isinstance(other, int):
            return self.value != other
        else:
            return self.value != other.value

    def __le__(self, other):
        if isinstance(other, int):
            return self.value <= other
        else:
            return self.value <= other.value

    def __ge__(self, other):
        if isinstance(other, int):
            return self.value >= other
        else:
            return self.value >= other.value

    def __lt__(self, other):
        if isinstance(other, int):
            return self.value < other
        else:
            return self.value < other.value

    def __gt__(self, other):
        if isinstance(other, int):
            return self.value > other
        else:
            return self.value > other.value

    def __add__(self, other):
        if isinstance(other, Nat):
            return NatMut(self.value + other)
        else:
            return NatMut(self.value + other.value)

    def __radd__(self, other):
        if isinstance(other, Nat):
            return Nat(other + self.value)
        else:
            return Nat(other.value + self.value)

    def __mul__(self, other):
        if isinstance(other, Nat):
            return NatMut(self.value * other)
        else:
            return NatMut(self.value * other.value)

    def __rmul__(self, other):
        if isinstance(other, Nat):
            return Nat(other * self.value)
        else:
            return Nat(other.value * self.value)

    def __pow__(self, other):
        if isinstance(other, Nat):
            return NatMut(self.value**other)
        else:
            return NatMut(self.value**other.value)

    def __pos__(self):
        return self

    def try_new(i):  # -> Result[Nat]
        if i >= 0:
            return NatMut(i)
        else:
            return Error("Nat can't be negative")

    def times(self, f):
        for _ in range(self.value):
            f()
