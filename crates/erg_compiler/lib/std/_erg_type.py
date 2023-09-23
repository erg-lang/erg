from typing import Union

class UnionType:
        __origin__ = Union
        __args__: list # list[type]
        def __init__(self, *args):
            self.__args__ = args

class FakeGenericAlias:
        __origin__: type
        __args__: list # list[type]
        def __init__(self, origin, *args):
            self.__origin__ = origin
            self.__args__ = args
try:
    from types import GenericAlias
except ImportError:
    GenericAlias = FakeGenericAlias

def is_type(x) -> bool:
    return isinstance(x, (type, GenericAlias, UnionType))

# The behavior of `builtins.isinstance` depends on the Python version.
def _isinstance(obj, classinfo) -> bool:
    if isinstance(classinfo, (GenericAlias, UnionType)):
        if classinfo.__origin__ == Union:
            return any(isinstance(obj, t) for t in classinfo.__args__)
        else:
            return isinstance(obj, classinfo.__origin__)
    elif is_type(classinfo):
        return isinstance(obj, classinfo)
    else:
        return False
