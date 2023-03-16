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
