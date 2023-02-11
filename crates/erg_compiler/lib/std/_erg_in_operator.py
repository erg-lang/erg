from _erg_result import is_ok
from _erg_range import Range

def in_operator(elem, y):
    if type(y) == type:
        if isinstance(elem, y):
            return True
        elif is_ok(y.try_new(elem)):
            return True
        # TODO: trait check
        return False
    elif issubclass(type(y), list) \
        and (type(y[0]) == type or issubclass(type(y[0]), Range)):
        # FIXME:
        type_check = in_operator(elem[0], y[0])
        len_check = len(elem) == len(y)
        return type_check and len_check
    elif issubclass(type(y), dict) and issubclass(type(next(iter(y.keys()))), type):
        # TODO:
        type_check = True # in_operator(x[next(iter(x.keys()))], next(iter(y.keys())))
        len_check = len(elem) >= len(y)
        return type_check and len_check
    else:
        return elem in y
