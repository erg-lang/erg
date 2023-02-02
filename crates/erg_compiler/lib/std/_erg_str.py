from _erg_result import Error
from _erg_int import Int

class Str(str):
    def __instancecheck__(cls, obj):
        return isinstance(obj, str)
    def try_new(s: str): # -> Result[Nat]
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

class StrMut(): # Inherits Str
    value: Str

    def __init__(self, s: str):
        self.value = s
    def __repr__(self):
        return self.value.__repr__()
    def __eq__(self, other):
        if isinstance(other, Str):
            return self.value == other
        else:
            return self.value == other.value
    def __ne__(self, other):
        if isinstance(other, Str):
            return self.value != other
        else:
            return self.value != other.value
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
        self.value = self.value[:idx] + self.value[idx+1:]
        return char
    def insert(self, idx: int, s: str):
        self.value = self.value[:idx] + s + self.value[idx:]
