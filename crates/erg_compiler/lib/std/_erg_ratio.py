from fractions import Fraction
from _erg_result import Error
from _erg_control import then__
from fractions import Fraction

__all__ = ["Ratio", "RatioMut"]


class Ratio(Fraction):
    __slots__ = ("numer", "denom")

    def try_new(i):
        if isinstance(i, Ratio):
            Ratio(i)
        else:
            Error("not a rational")

    def mutate(self):
        return RatioMut(self)

    def __str__(self) -> str:
        return super().__str__()

    def __repr__(self) -> str:
        return super().__repr__()

    def __add__(self, other):
        return then__(Fraction.__add__(self, other), Ratio)

    def __sub__(self, other):
        return then__(Fraction.__sub__(self, other), Ratio)

    def __rsub__(self, other):
        return then__(Fraction.__sub__(Fraction(other), self), Ratio)

    def __mul__(self, other):
        return then__(Fraction.__mul__(self, other), Ratio)

    def __rmul__(self, other):
        return then__(Fraction.__mul__(Fraction(other), self), Ratio)

    def __div__(self, other):
        return then__(Fraction.__div__(self, other), Ratio)

    def __rdiv__(self, other):
        return then__(Fraction.__div__(Fraction(other), self), Ratio)

    def __floordiv__(self, other):
        return then__(Fraction.__floordiv__(self, other), Ratio)

    def __rfloordiv__(self, other):
        return then__(Fraction.__floordiv__(Fraction(other), self), Ratio)

    def __pow__(self, other):
        return then__(Fraction.__pow__(self, other), Ratio)

    def __rpow__(self, other):
        return then__(Fraction.__pow__(Fraction(other), self), Ratio)

    def __pos__(self):
        return self

    def __neg__(self):
        return then__(Fraction.__neg__(self), Ratio)


class RatioMut:
    value: Ratio

    def __init__(self, r) -> None:
        self.value = Ratio(r)

    def __repr__(self):
        return self.value.__repr__()

    def __hash__(self):
        return self.value.__hash__()

    def __deref__(self):
        return self.value

    def __eq__(self, other):
        if isinstance(other, Ratio):
            return self.value == other
        else:
            return self.value == other.value

    def __ne__(self, other):
        if isinstance(other, Ratio):
            return self.value != other
        else:
            return self.value != other.value

    def __le__(self, other):
        if isinstance(other, Ratio):
            return self.value <= other
        else:
            return self.value <= other.value

    def __ge__(self, other):
        if isinstance(other, Ratio):
            return self.value >= other
        else:
            return self.value >= other.value

    def __lt__(self, other):
        if isinstance(other, Ratio):
            return self.value < other
        else:
            return self.value < other.value

    def __gt__(self, other):
        if isinstance(other, Ratio):
            return self.value > other
        else:
            return self.value > other.value

    def __add__(self, other):
        if isinstance(other, Ratio):
            return RatioMut(self.value + other)
        else:
            return RatioMut(self.value + other.value)

    def __sub__(self, other):
        if isinstance(other, Ratio):
            return RatioMut(self.value - other)
        else:
            return RatioMut(self.value - other.value)

    def __mul__(self, other):
        if isinstance(other, Ratio):
            return RatioMut(self.value * other)
        else:
            return RatioMut(self.value * other.value)

    def __floordiv__(self, other):
        if isinstance(other, Ratio):
            return RatioMut(self.value // other)
        else:
            return RatioMut(self.value // other.value)

    def __pow__(self, other):
        if isinstance(other, Ratio):
            return RatioMut(self.value**other)
        else:
            return RatioMut(self.value**other.value)

    def __pos__(self):
        return self

    def __neg__(self):
        return RatioMut(-self.value)
