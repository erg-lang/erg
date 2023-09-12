from typing import _GenericAlias, Union
try:
    from types import UnionType
except ImportError:
    class UnionType:
        __args__: list # list[type]
        def __init__(self, *args):
            self.__args__ = args

def is_type(x) -> bool:
    return isinstance(x, type) or \
        isinstance(x, _GenericAlias) or \
        isinstance(x, UnionType)

instanceof = isinstance
# The behavior of `builtins.isinstance` depends on the Python version.
def isinstance(obj, classinfo) -> bool:
    if instanceof(classinfo, _GenericAlias) and classinfo.__origin__ == Union:
        return any(instanceof(obj, t) for t in classinfo.__args__)
    else:
        return instanceof(obj, classinfo)
