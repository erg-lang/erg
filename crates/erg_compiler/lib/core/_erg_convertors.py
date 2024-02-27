from _erg_float import Float
from _erg_int import Int
from _erg_nat import Nat
from _erg_str import Str


def int__(i):
    return Int(i)


def nat__(i):
    return Nat(i)


def float__(f):
    return Float(f)


def str__(s):
    return Str(s)
