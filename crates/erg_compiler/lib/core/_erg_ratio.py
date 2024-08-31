from fractions import Fraction

from _erg_control import then__
from _erg_result import Error
from _erg_type import MutType
from _erg_float import FloatMut


class Ratio(Fraction):
    FRAC_ZERO = Fraction(0)

    def __new__(cls, fraction):
        if isinstance(fraction, (int, float, Fraction)):
            return super().__new__(cls, fraction)

        numerator, denominator = fraction
        if isinstance(numerator, (int, float, Fraction)) and isinstance(
            denominator, (int, float, Fraction)
        ):
            if (numerator, denominator) == (cls.FRAC_ZERO, cls.FRAC_ZERO):
                return super().__new__(cls, cls.FRAC_ZERO)
            return super().__new__(cls, numerator, denominator)
        else:
            raise ValueError("This class only accepts the fraction")

    def try_new(numerator: int, denominator: int):
        if isinstance(numerator, int) and isinstance(denominator, int):
            return Ratio((numerator, denominator))
        else:
            return Error("not an integer")

    def bit_count(self):
        if hasattr(int, "bit_count"):
            return int.bit_count(self)
        else:
            return bin(self).count("1")

    def succ(self):
        return Ratio(self) + 1

    def pred(self):
        return Ratio(self) - 1

    def mutate(self):
        return RatioMut(self)

    def __repr__(self):
        return f"Ratio({self.numerator}/{self.denominator})"

    def __str__(self):
        return f"{self.numerator}/{self.denominator}"

    def __add__(self, other):
        return then__(super().__add__(other), Ratio)

    def __sub__(self, other):
        return then__(super().__sub__(other), Ratio)

    def __mul__(self, other):
        return then__(super().__mul__(other), Ratio)

    def __mod__(self, other):
        return then__(super().__mod__(other), Ratio)

    def __truediv__(self, other):
        return then__(super().__truediv__(other), Ratio)

    def __floordiv__(self, other):
        return then__(super().__floordiv__(other), Ratio)

    def __pow__(self, other):
        return float(self).__pow__(float(other))

    def __rpow__(self, other):
        return float(self).__rpow__(float(other))

    def __pos__(self):
        return self

    def __neg__(self):
        return then__(super().__neg__(), Ratio)

    def __float__(self):
        return super().__float__()


class RatioMut(MutType):
    value: Ratio

    def __init__(self, fraction):
        self.value = Ratio(fraction)

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
            return RatioMut(self.value + other.value)
        else:
            return RatioMut(self.value + other)

    def __sub__(self, other):
        if isinstance(other, MutType):
            return RatioMut(self.value - other.value)
        else:
            return RatioMut(self.value - other)

    def __mul__(self, other):
        if isinstance(other, MutType):
            return RatioMut(self.value * other.value)
        else:
            return RatioMut(self.value * other)

    def __mod__(self, other):
        if isinstance(other, MutType):
            return RatioMut(self.value % other.value)
        else:
            return RatioMut(self.value % other)

    def __floordiv__(self, other):
        if isinstance(other, MutType):
            return RatioMut(self.value // other.value)
        else:
            return RatioMut(self.value // other)

    def __truediv__(self, other):
        if isinstance(other, MutType):
            return RatioMut(self.value / other.value)
        else:
            return RatioMut(self.value / other)

    def __pow__(self, other):
        if isinstance(other, MutType):
            return FloatMut(self.value**other.value)
        else:
            return FloatMut(self.value**other)

    def __pos__(self):
        return self

    def __neg__(self):
        return RatioMut(-self.value)

    def update(self, f):
        self.value = Ratio(f(self.value))

    def inc(self, i=1):
        self.value = Ratio(self.value + i)

    def dec(self, i=1):
        self.value = Ratio(self.value - i)

    def succ(self):
        return self.value.succ()

    def pred(self):
        return self.value.pred()

    def copy(self):
        return RatioMut(self.value)
