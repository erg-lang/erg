from _erg_result import Error

class Str(str):
    def __instancecheck__(cls, obj):
        return isinstance(obj, str)
    def try_new(s: str): # -> Result[Nat]
        if isinstance(s, str):
            return Str(s)
        else:
            return Error("Str can't be other than str")
