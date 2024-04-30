from _erg_control import then__
from _erg_int import Int
from _erg_result import Error
from _erg_type import MutType


class Str(str):
    def __instancecheck__(cls, obj):
        return isinstance(obj, str)

    def try_new(s: str):  # -> Result[Nat]
        if isinstance(s, str):
            return Str(s)
        else:
            return Error("Str can't be other than str")

    def get(self, i: int):
        if len(self) > i:
            return Str(self[i])
        else:
            return None

    def mutate(self):
        return StrMut(self)

    def to_int(self):
        return Int(self) if self.isdigit() else None

    def contains(self, s):
        return s in self

    def __add__(self, other):
        return then__(str.__add__(self, other), Str)

    def __mul__(self, other):
        return then__(str.__mul__(self, other), Str)

    def __mod__(self, other):
        return then__(str.__mod__(other, self), Str)

    def __getitem__(self, index_or_slice):
        from _erg_range import Range

        if isinstance(index_or_slice, slice):
            return Str(str.__getitem__(self, index_or_slice))
        elif isinstance(index_or_slice, Range):
            return Str(str.__getitem__(self, index_or_slice.into_slice()))
        else:
            return str.__getitem__(self, index_or_slice)

    def from_(self, nth: int):
        return self[nth:]

class StrMut(MutType):  # Inherits Str
    value: Str

    def __init__(self, s: str):
        self.value = s

    def __repr__(self):
        return self.value.__repr__()

    def __str__(self):
        return self.value.__str__()

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

    def update(self, f):
        self.value = Str(f(self.value))

    def try_new(s: str):
        if isinstance(s, str):
            self = StrMut()
            self.value = s
            return self
        else:
            return Error("Str! can't be other than str")

    def clear(self):
        self.value = ""

    def pop(self):
        if len(self.value) > 0:
            last = self.value[-1]
            self.value = self.value[:-1]
            return last
        else:
            return Error("Can't pop from empty `Str!`")

    def push(self, s: str):
        self.value += s

    def remove(self, idx: int):
        char = self.value[idx]
        self.value = self.value[:idx] + self.value[idx + 1 :]
        return char

    def insert(self, idx: int, s: str):
        self.value = self.value[:idx] + s + self.value[idx:]

    def copy(self):
        return StrMut(self.value)
