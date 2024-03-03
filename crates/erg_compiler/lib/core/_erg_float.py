from _erg_control import then__
from _erg_result import Error
from _erg_type import MutType


class Float(float):
    EPSILON = 2.220446049250313e-16

    def try_new(i):  # -> Result[Nat]
        if isinstance(i, float):
            return Float(i)
        else:
            return Error("not a float")

    def mutate(self):
        return FloatMut(self)

    def __abs__(self):
        return Float(float.__abs__(self))

    def __add__(self, other):
        return then__(float.__add__(self, other), Float)

    def __sub__(self, other):
        return then__(float.__sub__(self, other), Float)

    def __mul__(self, other):
        return then__(float.__mul__(self, other), Float)

    def __div__(self, other):
        return then__(float.__div__(self, other), Float)

    def __floordiv__(self, other):
        return then__(float.__floordiv__(self, other), Float)

    def __truediv__(self, other):
        return then__(float.__truediv__(self, other), Float)

    def __pow__(self, other):
        return then__(float.__pow__(self, other), Float)

    def __rpow__(self, other):
        return then__(float.__pow__(float(other), self), Float)

    def __pos__(self):
        return self

    def __neg__(self):
        return then__(float.__neg__(self), Float)

    def nearly_eq(self, other, epsilon=EPSILON):
        return abs(self - other) < epsilon


class FloatMut(MutType):  # inherits Float
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

    def __float__(self):
        return self.value.__float__()

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
            return FloatMut(self.value + other.value)
        else:
            return FloatMut(self.value + other)

    def __sub__(self, other):
        if isinstance(other, MutType):
            return FloatMut(self.value - other.value)
        else:
            return FloatMut(self.value - other)

    def __mul__(self, other):
        if isinstance(other, MutType):
            return FloatMut(self.value * other.value)
        else:
            return FloatMut(self.value * other)

    def __floordiv__(self, other):
        if isinstance(other, MutType):
            return FloatMut(self.value // other.value)
        else:
            return FloatMut(self.value // other)

    def __truediv__(self, other):
        if isinstance(other, MutType):
            return FloatMut(self.value / other.value)
        else:
            return FloatMut(self.value / other)

    def __pow__(self, other):
        if isinstance(other, MutType):
            return FloatMut(self.value**other.value)
        else:
            return FloatMut(self.value**other)

    def __pos__(self):
        return self

    def __neg__(self):
        return FloatMut(-self.value)

    def update(self, f):
        self.value = Float(f(self.value))

    def inc(self, value=1.0):
        self.value = Float(self.value + value)

    def dec(self, value=1.0):
        self.value = Float(self.value - value)

    def copy(self):
        return FloatMut(self.value)
