from _erg_nat import Nat
from _erg_str import Str

# from collections.abc import Iterable, Sequence, Iterator, Container


class Range:
    def __init__(self, start, end):
        self.start = start
        self.end = end

    def __contains__(self, item):
        pass

    @staticmethod
    def from_slice(slice):
        pass

    def into_slice(self):
        pass

    def __getitem__(self, item):
        res = self.start + item
        if res in self:
            return res
        else:
            raise IndexError("Index out of range")

    # TODO: for Str, etc.
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


# Sequence.register(Range)
# Container.register(Range)
# Iterable.register(Range)


# represents `start<..end`
class LeftOpenRange(Range):
    def __contains__(self, item):
        return self.start < item <= self.end


# represents `start..<end`
class RightOpenRange(Range):
    def __contains__(self, item):
        return self.start <= item < self.end

    @staticmethod
    def from_slice(slice):
        return Range(slice.start, slice.stop)

    def into_slice(self):
        return slice(self.start, self.end)


# represents `start<..<end`
class OpenRange(Range):
    def __contains__(self, item):
        return self.start < item < self.end


# represents `start..end`
class ClosedRange(Range):
    def __contains__(self, item):
        return self.start <= item <= self.end

    @staticmethod
    def from_slice(slice):
        return Range(slice.start, slice.stop - 1)

    def into_slice(self):
        return slice(self.start, self.end + 1)


class RangeIterator:
    def __init__(self, rng):
        self.rng = rng
        self.needle = self.rng.start
        if issubclass(Nat, type(self.rng.start)):
            if not (self.needle in self.rng):
                self.needle += 1
        elif issubclass(Str, type(self.rng.start)):
            if not (self.needle in self.rng):
                self.needle = chr(ord(self.needle) + 1)
        else:
            if not (self.needle in self.rng):
                self.needle = self.needle.succ()

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
                self.needle = self.needle.succ()
                return result
        raise StopIteration


# Iterator.register(RangeIterator)
