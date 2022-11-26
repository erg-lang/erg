from _erg_result import Error

class Nat(int):
    def try_new(i: int): # -> Result[Nat]
        if i >= 0:
            return Nat(i)
        else:
            return Error("Nat can't be negative")

    def times(self, f):
        for _ in range(self):
            f()
