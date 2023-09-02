from _erg_result import is_ok
from _erg_range import Range

from collections import namedtuple

# (elem in y) == contains_operator(y, elem)
def contains_operator(y, elem) -> bool:
    if hasattr(elem, "type_check"):
        return elem.type_check(y)
    # 1 in Int
    elif type(y) == type:
        if isinstance(elem, y):
            return True
        elif hasattr(y, "try_new") and is_ok(y.try_new(elem)):
            return True
        # TODO: trait check
        return False
    # [1] in [Int]
    elif isinstance(y, list) and isinstance(elem, list) and (
        type(y[0]) == type or isinstance(y[0], Range)
    ):
        # FIXME:
        type_check = contains_operator(y[0], elem[0])
        len_check = len(elem) == len(y)
        return type_check and len_check
    # (1, 2) in (Int, Int)
    elif isinstance(y, tuple) and isinstance(elem, tuple) and (
        type(y[0]) == type or isinstance(y[0], Range)
    ):
        if not hasattr(elem, "__iter__"):
            return False
        type_check = all(map(lambda x: contains_operator(x[0], x[1]), zip(y, elem)))
        len_check = len(elem) == len(y)
        return type_check and len_check
    # {1: 2} in {Int: Int}
    elif isinstance(y, dict) and isinstance(elem, dict) and isinstance(next(iter(y.keys())), type):
        # TODO:
        type_check = True  # contains_operator(next(iter(y.keys())), x[next(iter(x.keys()))])
        len_check = len(elem) >= len(y)
        return type_check and len_check
    elif isinstance(elem, list):
        from _erg_array import Array
        return contains_operator(y, Array(elem))
    else:
        return elem in y
