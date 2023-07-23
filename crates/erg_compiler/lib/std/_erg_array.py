from _erg_control import then__
from _erg_range import Range

class Array(list):
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
        elif isinstance(index_or_slice, Range):
            return Array(list.__getitem__(self, index_or_slice.into_slice()))
        else:
            return list.__getitem__(self, index_or_slice)
