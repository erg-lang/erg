from _erg_int import Int
from _erg_nat import Nat

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
