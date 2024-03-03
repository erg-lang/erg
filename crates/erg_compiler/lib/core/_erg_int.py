from _erg_control import then__
from _erg_result import Error
from _erg_type import MutType


class Int(int):
    def try_new(i):  # -> Result[Nat]
        if isinstance(i, int):
            return Int(i)
        else:
            return Error("not an integer")

    def bit_count(self):
        if hasattr(int, "bit_count"):
            return int.bit_count(self)
        else:
            return bin(self).count("1")

    def succ(self):
        return Int(self + 1)

    def pred(self):
        return Int(self - 1)

    def mutate(self):
        return IntMut(self)

    def __add__(self, other):
        return then__(int.__add__(self, other), Int)

    def __sub__(self, other):
        return then__(int.__sub__(self, other), Int)

    def __mul__(self, other):
        return then__(int.__mul__(self, other), Int)

    def __div__(self, other):
        return then__(int.__div__(self, other), Int)

    def __floordiv__(self, other):
        return then__(int.__floordiv__(self, other), Int)

    def __pow__(self, other):
        return then__(int.__pow__(self, other), Int)

    def __rpow__(self, other):
        return then__(int.__pow__(other, self), Int)

    def __pos__(self):
        return self

    def __neg__(self):
        return then__(int.__neg__(self), Int)


class IntMut(MutType):  # inherits Int
    value: Int

    def __init__(self, i):
        self.value = Int(i)

    def __int__(self):
        return self.value.__int__()

    def __float__(self):
        return self.value.__float__()

    def __repr__(self):
        return self.value.__repr__()

    def __hash__(self):
        return self.value.__hash__()

    def __eq__(self, other):
        if isinstance(other, MutType):
            return self.value == other.value
        else:
            return self.value == other

    def __ne__(self, other):
        if isinstance(other, MutType):
            return self.value != other.value
        else:
            return self.value != other

    def __le__(self, other):
        if isinstance(other, MutType):
            return self.value <= other.value
        else:
            return self.value <= other

    def __ge__(self, other):
        if isinstance(other, MutType):
            return self.value >= other.value
        else:
            return self.value >= other

    def __lt__(self, other):
        if isinstance(other, MutType):
            return self.value < other.value
        else:
            return self.value < other

    def __gt__(self, other):
        if isinstance(other, MutType):
            return self.value > other.value
        else:
            return self.value > other

    def __add__(self, other):
        if isinstance(other, MutType):
            return IntMut(self.value + other.value)
        else:
            return IntMut(self.value + other)

    def __sub__(self, other):
        if isinstance(other, MutType):
            return IntMut(self.value - other.value)
        else:
            return IntMut(self.value - other)

    def __mul__(self, other):
        if isinstance(other, MutType):
            return IntMut(self.value * other.value)
        else:
            return IntMut(self.value * other)

    def __floordiv__(self, other):
        if isinstance(other, MutType):
            return IntMut(self.value // other.value)
        else:
            return IntMut(self.value // other)

    def __truediv__(self, other):
        if isinstance(other, MutType):
            return IntMut(self.value / other.value)
        else:
            return IntMut(self.value / other)

    def __pow__(self, other):
        if isinstance(other, MutType):
            return IntMut(self.value**other.value)
        else:
            return IntMut(self.value**other)

    def __pos__(self):
        return self

    def __neg__(self):
        return IntMut(-self.value)

    def update(self, f):
        self.value = Int(f(self.value))

    def inc(self, i=1):
        self.value = Int(self.value + i)

    def dec(self, i=1):
        self.value = Int(self.value - i)

    def succ(self):
        return self.value.succ()

    def pred(self):
        return self.value.pred()

    def copy(self):
        return IntMut(self.value)
