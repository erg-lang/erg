from _erg_contains_operator import contains_operator
from _erg_control import then__
from _erg_int import IntMut
from _erg_nat import NatMut
from _erg_range import Range
from _erg_result import Error
from _erg_result import is_ok
from _erg_type import UnionType


class Array(list):
    @staticmethod
    def try_new(arr):  # -> Result[Array]
        if isinstance(arr, list):
            return Array(arr)
        else:
            return Error("not a list")

    def generic_try_new(arr, cls=None):  # -> Result[Array]
        if cls is None:
            return Array.try_new(arr)
        else:
            elem_t = cls.__args__[0]
            elems = []
            for elem in arr:
                if not hasattr(elem_t, "try_new"):
                    return Error("not a " + str(elem_t))
                # TODO: nested check
                elem = elem_t.try_new(elem)
                if is_ok(elem):
                    elems.append(elem)
                else:
                    return Error("not a " + str(elem_t))
            return Array(elems)

    def dedup(self, same_bucket=None):
        if same_bucket is None:
            return Array(list(set(self)))
        else:
            removes = []
            for lhs, rhs in zip(self, self[1:]):
                if same_bucket(lhs, rhs):
                    removes.append(lhs)
            for remove in removes:
                self.remove(remove)
            return self

    def get(self, index, default=None):
        try:
            return self[index]
        except IndexError:
            return default

    def push(self, value):
        self.append(value)
        return self

    def partition(self, f):
        return Array(list(filter(f, self))), Array(
            list(filter(lambda x: not f(x), self))
        )

    def __mul__(self, n):
        return then__(list.__mul__(self, n), Array)

    def __getitem__(self, index_or_slice):
        if isinstance(index_or_slice, slice):
            return Array(list.__getitem__(self, index_or_slice))
        elif isinstance(index_or_slice, NatMut) or isinstance(index_or_slice, IntMut):
            return list.__getitem__(self, int(index_or_slice))
        elif isinstance(index_or_slice, Range):
            return Array(list.__getitem__(self, index_or_slice.into_slice()))
        else:
            return list.__getitem__(self, index_or_slice)

    def __hash__(self):
        return hash(tuple(self))

    def update(self, f):
        self = Array(f(self))

    def type_check(self, t: type) -> bool:
        if isinstance(t, list):
            if len(t) < len(self):
                return False
            for inner_t, elem in zip(t, self):
                if not contains_operator(inner_t, elem):
                    return False
            return True
        elif isinstance(t, set):
            return self in t
        elif isinstance(t, UnionType):
            return any([self.type_check(_t) for _t in t.__args__])
        elif not hasattr(t, "__args__"):
            return isinstance(self, t)
        elem_t = t.__args__[0]
        l = None if len(t.__args__) != 2 else t.__args__[1]
        if l is not None and l != len(self):
            return False
        for elem in self:
            if not contains_operator(elem_t, elem):
                return False
        return True

    def update_nth(self, index, f):
        self[index] = f(self[index])

    def sum(self, start=0):
        return sum(self, start)

    def prod(self, start=1):
        from functools import reduce

        return reduce(lambda x, y: x * y, self, start)

    def reversed(self):
        return Array(list.__reversed__(self))

    def insert_at(self, index, value):
        self.insert(index, value)
        return self

    def remove_at(self, index):
        del self[index]
        return self

    def remove_all(self, item):
        while item in self:
            self.remove(item)
        return self

    def repeat(self, n):
        from copy import deepcopy

        new = []
        for _ in range(n):
            new.extend(deepcopy(self))
        return Array(new)


class UnsizedArray:
    elem: object

    def __init__(self, elem):
        self.elem = elem
