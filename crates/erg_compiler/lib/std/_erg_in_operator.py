from _erg_result import is_ok
from _erg_range import Range

from collections import namedtuple

def in_operator(elem, y):
    if type(y) == type:
        if isinstance(elem, y):
            return True
        elif hasattr(y, "try_new") and is_ok(y.try_new(elem)):
            return True
        # TODO: trait check
        return False
    elif isinstance(y, list) and (
        type(y[0]) == type or isinstance(y[0], Range)
    ):
        # FIXME:
        type_check = in_operator(elem[0], y[0])
        len_check = len(elem) == len(y)
        return type_check and len_check
    elif isinstance(y, tuple):
        if not hasattr(elem, "__iter__"):
            return False
        type_check = all(map(lambda x: in_operator(x[0], x[1]), zip(elem, y)))
        len_check = len(elem) == len(y)
        return type_check and len_check
    elif isinstance(y, dict) and isinstance(next(iter(y.keys())), type):
        # TODO:
        type_check = True  # in_operator(x[next(iter(x.keys()))], next(iter(y.keys())))
        len_check = len(elem) >= len(y)
        return type_check and len_check
    else:
        return elem in y
