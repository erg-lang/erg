from _erg_nat import Nat
from _erg_nat import NatMut
from _erg_result import Error
from _erg_type import MutType


class Bool(Nat):
    def try_new(b: bool):  # -> Result[Nat]
        if isinstance(b, bool):
            return Bool(b)
        else:
            return Error("Bool can't be other than True or False")

    def __str__(self) -> str:
        if self:
            return "True"
        else:
            return "False"

    def __repr__(self) -> str:
        return self.__str__()

    def mutate(self):
        return BoolMut(self)

    def invert(self):
        return Bool(not self)


class BoolMut(NatMut):
    value: Bool

    def __init__(self, b: Bool):
        self.value = b

    def __repr__(self):
        return self.value.__repr__()

    def __bool__(self):
        return bool(self.value)

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
        self.value = Bool(f(self.value))

    def invert(self):
        self.value = self.value.invert()

    def copy(self):
        return BoolMut(self.value)
