class Array(list):
    def dedup(self, f=None):
        if f == None:
            return Array(list(set(self)))
        else:
            return Array(list(set(map(f, self))))
    def push(self, value):
        self.append(value)
        return self
    def partition(self, f):
        return Array(list(filter(f, self))), Array(list(filter(lambda x: not f(x), self)))
