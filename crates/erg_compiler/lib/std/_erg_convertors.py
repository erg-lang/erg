from _erg_int import Int
from _erg_nat import Nat
from _erg_float import Float
from _erg_str import Str


def int__(i):
    try:
        return Int(i)
    except:
        return None


def nat__(i):
    try:
        return Nat(i)
    except:
        return None


def float__(f):
    try:
        return Float(f)
    except:
        return None


def str__(s):
    try:
        return Str(s)
    except:
        return None
