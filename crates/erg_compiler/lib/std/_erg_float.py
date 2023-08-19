from _erg_result import Error
from _erg_control import then__


class Float(float):
    EPSILON = 2.220446049250313e-16

    def try_new(i):  # -> Result[Nat]
        if isinstance(i, float):
            return Float(i)
        else:
            return Error("not a float")

    def mutate(self):
        return FloatMut(self)

    def __add__(self, other):
        return then__(float.__add__(self, other), Float)

    def __radd__(self, other):
        return then__(float.__add__(float(other), self), Float)

    def __sub__(self, other):
        return then__(float.__sub__(self, other), Float)

    def __rsub__(self, other):
        return then__(float.__sub__(float(other), self), Float)

    def __mul__(self, other):
        return then__(float.__mul__(self, other), Float)

    def __rmul__(self, other):
        return then__(float.__mul__(float(other), self), Float)

    def __div__(self, other):
        return then__(float.__div__(self, other), Float)

    def __rdiv__(self, other):
        return then__(float.__div__(float(other), self), Float)

    def __floordiv__(self, other):
        return then__(float.__floordiv__(self, other), Float)

    def __rfloordiv__(self, other):
        return then__(float.__floordiv__(float(other), self), Float)

    def __pow__(self, other):
        return then__(float.__pow__(self, other), Float)

    def __rpow__(self, other):
        return then__(float.__pow__(float(other), self), Float)

    def __pos__(self):
        return self

    def __neg__(self):
        return then__(float.__neg__(self), Float)


class FloatMut:  # inherits Float
    value: Float

    EPSILON = 2.220446049250313e-16

    def __init__(self, i):
        self.value = Float(i)

    def __repr__(self):
        return self.value.__repr__()

    def __hash__(self):
        return self.value.__hash__()

    def __deref__(self):
        return self.value

    def __eq__(self, other):
        if isinstance(other, Float):
            return self.value == other
        else:
            return self.value == other.value

    def __ne__(self, other):
        if isinstance(other, Float):
            return self.value != other
        else:
            return self.value != other.value

    def __le__(self, other):
        if isinstance(other, Float):
            return self.value <= other
        else:
            return self.value <= other.value

    def __ge__(self, other):
        if isinstance(other, Float):
            return self.value >= other
        else:
            return self.value >= other.value

    def __lt__(self, other):
        if isinstance(other, Float):
            return self.value < other
        else:
            return self.value < other.value

    def __gt__(self, other):
        if isinstance(other, Float):
            return self.value > other
        else:
            return self.value > other.value

    def __add__(self, other):
        if isinstance(other, Float):
            return FloatMut(self.value + other)
        else:
            return FloatMut(self.value + other.value)

    def __sub__(self, other):
        if isinstance(other, Float):
            return FloatMut(self.value - other)
        else:
            return FloatMut(self.value - other.value)

    def __mul__(self, other):
        if isinstance(other, Float):
            return FloatMut(self.value * other)
        else:
            return FloatMut(self.value * other.value)

    def __floordiv__(self, other):
        if isinstance(other, Float):
            return FloatMut(self.value // other)
        else:
            return FloatMut(self.value // other.value)

    def __pow__(self, other):
        if isinstance(other, Float):
            return FloatMut(self.value**other)
        else:
            return FloatMut(self.value**other.value)

    def __pos__(self):
        return self

    def __neg__(self):
        return FloatMut(-self.value)
