class Dict(dict):
    def concat(self, other):
        return Dict({**self, **other})
    def diff(self, other):
        return Dict({k: v for k, v in self.items() if k not in other})
    # other: Iterable
    def extend(self, other):
        self.update(other)
    # other: Dict
    def merge(self, other):
        self.update(other)
    def insert(self, key, value):
        self[key] = value
    def remove(self, key):
        res = self.get(key)
        if res != None:
            del self[key]
        return res
