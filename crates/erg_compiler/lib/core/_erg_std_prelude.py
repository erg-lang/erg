# HACK: import MutType to suppress segfault in CPython 3.10 (cause unknown)
from _erg_list import List, UnsizedList
from _erg_bool import Bool
from _erg_bytes import Bytes
from _erg_contains_operator import contains_operator
from _erg_dict import Dict
from _erg_float import Float, FloatMut
from _erg_int import Int, IntMut
from _erg_mutate_operator import mutate_operator
from _erg_nat import Nat, NatMut
from _erg_range import (ClosedRange, LeftOpenRange, OpenRange, Range,
                        RangeIterator, RightOpenRange)
from _erg_result import Error, is_ok
from _erg_set import Set
from _erg_str import Str, StrMut
from _erg_type import MutType as _MutType
from _erg_iterable import (iterable_map, iterable_filter, iterable_reduce,
                        iterable_nth, iterable_skip, iterable_all,
                        iterable_any, iterable_position, iterable_find,
                        iterable_chain)

Record = tuple


class Never:
    pass

from typing import Generic, TypeVar

Ty = TypeVar('Ty')
M = TypeVar('M', bound=int)
L = TypeVar("L", bound=int)
T = TypeVar("T", bound=int)
I = TypeVar("I", bound=int)
Θ = TypeVar("Θ", bound=int)
N = TypeVar("N", bound=int)
J = TypeVar("J", bound=int)
class Dimension(Generic[Ty, M, L, T, I, Θ, N, J]):
    val: float
    def __init__(self, val: float):
        self.val = val
    def __float__(self):
        return float(self.val)
    def __int__(self):
        return int(self.val)
    def __str__(self):
        return f"Dimension({self.val})"
    def __add__(self, other):
        return Dimension(self.val + other)
    def __radd__(self, other):
        return Dimension(other + self.val)
    def __sub__(self, other):
        return Dimension(self.val - other)
    def __rsub__(self, other):
        return Dimension(other - self.val)
    def __mul__(self, other):
        return Dimension(self.val * other)
    def __rmul__(self, other):
        return Dimension(other * self.val)
    def __truediv__(self, other):
        return Dimension(self.val / other)
    def __floordiv__(self, other):
        return Dimension(self.val // other)
    def __rtruediv__(self, other):
        return Dimension(other / self.val)
    def __rfloordiv__(self, other):
        return Dimension(other // self.val)
    def __eq__(self, other):
        return self.val == other.val
    def __ne__(self, other):
        return self.val != other.val
    def __lt__(self, other):
        return self.val < other.val
    def __le__(self, other):
        return self.val <= other.val
    def __gt__(self, other):
        return self.val > other.val
    def __ge__(self, other):
        return self.val >= other.val
    def value(self):
        return self.val
    def type_check(self, t: type) -> bool:
        return t.__name__ == "Dimension"
