from _erg_result import Error
from _erg_control import then__

class Int(int):
    def try_new(i): # -> Result[Nat]
        if isinstance(i, int):
            Int(i)
        else:
            Error("not an integer")
    def succ(self):
        return Int(self + 1)
    def pred(self):
        return Int(self - 1)
    def mutate(self):
        return IntMut(self)
    def __add__(self, other):
        return then__(super().__add__(other), Int)
    def __radd__(self, other):
        return then__(super().__radd__(other), Int)
    def __sub__(self, other):
        return then__(super().__sub__(other), Int)
    def __rsub__(self, other):
        return then__(super().__rsub__(other), Int)
    def __mul__(self, other):
        return then__(super().__mul__(other), Int)
    def __rmul__(self, other):
        return then__(super().__rmul__(other), Int)
    def __div__(self, other):
        return then__(super().__div__(other), Int)
    def __rdiv__(self, other):
        return then__(super().__rdiv__(other), Int)
    def __floordiv__(self, other):
        return then__(super().__floordiv__(other), Int)
    def __rfloordiv__(self, other):
        return then__(super().__rfloordiv__(other), Int)
    def __pow__(self, other):
        return then__(super().__pow__(other), Int)
    def __rpow__(self, other):
        return then__(super().__rpow__(other), Int)

class IntMut(): # inherits Int
    value: Int

    def __init__(self, i):
        self.value = Int(i)
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
        if isinstance(other, Int):
            return IntMut(self.value + other)
        else:
            return IntMut(self.value + other.value)
    def __sub__(self, other):
        if isinstance(other, Int):
            return IntMut(self.value - other)
        else:
            return IntMut(self.value - other.value)
    def __mul__(self, other):
        if isinstance(other, Int):
            return IntMut(self.value * other)
        else:
            return IntMut(self.value * other.value)
    def __floordiv__(self, other):
        if isinstance(other, Int):
            return IntMut(self.value // other)
        else:
            return IntMut(self.value // other.value)
    def __pow__(self, other):
        if isinstance(other, Int):
            return IntMut(self.value ** other)
        else:
            return IntMut(self.value ** other.value)
    def inc(self, i=1):
        self.value = Int(self.value + i)
    def dec(self, i=1):
        self.value = Int(self.value - i)
    def succ(self):
        return self.value.succ()
    def pred(self):
        return self.value.pred()
