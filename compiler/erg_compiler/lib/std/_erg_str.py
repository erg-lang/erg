from _erg_result import Error

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

class StrMut(Str):
    def try_new(s: str):
        if isinstance(s, str):
            return StrMut(s)
        else:
            return Error("Str! can't be other than str")
    def clear(self):
        self = ""
    def pop(self):
        if len(self) > 0:
            last = self[-1]
            self = self[:-1]
            return last
        else:
            return Error("Can't pop from empty `Str!`")
    def push(self, c: str):
        self += c
    def remove(self, idx: int):
        char = self[idx]
        self = self[:idx] + self[idx+1:]
        return char
    def insert(self, idx: int, c: str):
        self = self[:idx] + c + self[idx:]
