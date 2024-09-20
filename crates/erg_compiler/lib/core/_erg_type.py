try:
    from typing import Union
except ImportError:
    import warnings

    warnings.warn("`typing.Union` is not available. Please use Python 3.8+.")

    class Union:
        pass


class UnionType:
    __origin__ = Union
    __args__: list  # list[type]

    def __init__(self, *args):
        self.__args__ = args

    def __str__(self):
        s = "UnionType[" + ", ".join(str(arg) for arg in self.__args__) + "]"
        return s

    def __repr__(self):
        return self.__str__()


class FakeGenericAlias:
    __name__: str
    __origin__: type
    __args__: list  # list[type]

    def __init__(self, origin, *args):
        self.__name__ = origin.__name__
        self.__origin__ = origin
        self.__args__ = args


try:
    from types import GenericAlias
except ImportError:
    GenericAlias = FakeGenericAlias


def is_type(x) -> bool:
    return isinstance(x, (type, FakeGenericAlias, GenericAlias, UnionType))


# The behavior of `builtins.isinstance` depends on the Python version.
def _isinstance(obj, classinfo) -> bool:
    if isinstance(classinfo, (FakeGenericAlias, GenericAlias, UnionType)):
        if classinfo.__origin__ == Union:
            return any(_isinstance(obj, t) for t in classinfo.__args__)
        else:
            return isinstance(obj, classinfo.__origin__)
    else:
        try:
            return isinstance(obj, classinfo)
        except:
            return False


class MutType:
    value: object

    # This method is a fallback to implement pseudo-inheritance.
    def __getattr__(self, name):
        return object.__getattribute__(self.value, name)
