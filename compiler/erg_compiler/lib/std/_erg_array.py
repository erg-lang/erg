class Array(list):
    def dedup(self):
        return Array(list(set(self)))
    def dedup_by(self, f):
        return Array(list(set(map(f, self))))
    def push(self, value):
        self.append(value)
        return self
