from collections.abc import Iterable, Sequence, Iterator, Container
from typing import TypeVar, Union, _SpecialForm, _type_check

class Error:
    def __init__(self, message):
        self.message = message

T = TypeVar("T")
@_SpecialForm
def Result(self, parameters):
    """Result type.

    Result[T] is equivalent to Union[T, Error].
    """
    arg = _type_check(parameters, f"{self} requires a single type.")
    return Union[arg, Error]

def is_ok(obj: Result[T]) -> bool:
    return not isinstance(obj, Error)

def in_operator(x, y):
    if type(y) == type:
        if isinstance(x, y):
            return True
        elif is_ok(y.try_new(x)):
            return True
        # TODO: trait check
        return False
    elif (type(y) == list or type(y) == set) \
        and (type(y[0]) == type or issubclass(type(y[0]), Range)):
        # FIXME:
        type_check = in_operator(x[0], y[0])
        len_check = len(x) == len(y)
        return type_check and len_check
    elif type(y) == dict and type(next(iter(y.keys()))) == type:
        # TODO:
        type_check = True # in_operator(x[next(iter(x.keys()))], next(iter(y.keys())))
        len_check = len(x) >= len(y)
        return type_check and len_check
    else:
        return x in y

class Nat(int):
    def try_new(i: int): # -> Result[Nat]
        if i >= 0:
            return Nat(i)
        else:
            return Error("Nat can't be negative")

    def times(self, f):
        for _ in range(self):
            f()

class Bool(Nat):
    def try_new(b: bool): # -> Result[Nat]
        if b == True or b == False:
            return Bool(b)
        else:
            return Error("Bool can't be other than True or False")

    def __str__(self) -> str:
        if self:
            return "True"
        else:
            return "False"
    def __repr__(self) -> str:
        return self.__str__()

class Str(str):
    def __instancecheck__(cls, obj):
        return obj == Str or obj == str

    def try_new(s: str): # -> Result[Nat]
        if isinstance(s, str):
            return Str(s)
        else:
            return Error("Str can't be other than str")

class Range:
    def __init__(self, start, end):
        self.start = start
        self.end = end
    def __contains__(self, item):
        pass
    def __getitem__(self, item):
        res = self.start + item
        if res in self:
            return res
        else:
            raise IndexError("Index out of range")
    def __len__(self):
        if self.start in self:
            if self.end in self:
                # len(1..4) == 4
                return self.end - self.start + 1
            else:
                # len(1..<4) == 3
                return self.end - self.start
        else:
            if self.end in self:
                # len(1<..4) == 3
                return self.end - self.start
            else:
                # len(1<..<4) == 2
                return self.end - self.start - 2
    def __iter__(self):
        return RangeIterator(rng=self)

Sequence.register(Range)
Container.register(Range)
Iterable.register(Range)

# represents `start<..end`
class LeftOpenRange(Range):
    def __contains__(self, item):
        return self.start < item <= self.end

# represents `start..<end`
class RightOpenRange(Range):
    def __contains__(self, item):
        return self.start <= item < self.end

# represents `start<..<end`
class OpenRange(Range):
    def __contains__(self, item):
        return self.start < item < self.end

# represents `start..end`
class ClosedRange(Range):
    def __contains__(self, item):
        return self.start <= item <= self.end

class RangeIterator:
    def __init__(self, rng):
        self.rng = rng
        self.needle = self.rng.start
        if issubclass(Nat, type(self.rng.start)):
            if not(self.needle in self.rng):
                self.needle += 1
        elif issubclass(Str, type(self.rng.start)):
            if not(self.needle in self.rng):
                self.needle = chr(ord(self.needle) + 1)
        else:
            if not(self.needle in self.rng):
                self.needle = self.needle.incremented()

    def __iter__(self):
        return self

    def __next__(self):
        if issubclass(Nat, type(self.rng.start)):
            if self.needle in self.rng:
                result = self.needle
                self.needle += 1
                return result
        elif issubclass(Str, type(self.rng.start)):
            if self.needle in self.rng:
                result = self.needle
                self.needle = chr(ord(self.needle) + 1)
                return result
        else:
            if self.needle in self.rng:
                result = self.needle
                self.needle = self.needle.incremented()
                return result
        raise StopIteration

Iterator.register(RangeIterator)

class Array(list):
    def push(self, value):
        self.append(value)
        return self
