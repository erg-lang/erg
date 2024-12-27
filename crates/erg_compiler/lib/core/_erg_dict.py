from _erg_result import Error

class Dict(dict):
    @staticmethod
    def try_new(dic):  # -> Result[Dict]
        if isinstance(dic, dict):
            return Dict(dic)
        else:
            return Error("not a dict")

    def concat(self, other):
        return Dict({**self, **other})

    def diff(self, other):
        return Dict({k: v for k, v in self.items() if k not in other})

    # other: Iterable
    def update(self, other, conflict_resolver=None):
        if conflict_resolver is None:
            super().update(other)
        elif isinstance(other, dict):
            self.merge(other, conflict_resolver)
        else:
            for k, v in other:
                if k in self:
                    self[k] = conflict_resolver(self[k], v)
                else:
                    self[k] = v

    # other: Dict
    def merge(self, other, conflict_resolver=None):
        self.update(other, conflict_resolver)

    def insert(self, key, value):
        self[key] = value

    def as_record(self):
        from collections import namedtuple

        return namedtuple("Record", self.keys())(**self)
