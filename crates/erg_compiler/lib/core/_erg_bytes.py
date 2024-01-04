class Bytes(bytes):
    def try_new(*b):  # -> Result[Nat]
        return Bytes(bytes(*b))

    def __getitem__(self, index_or_slice):
        from _erg_range import Range
        if isinstance(index_or_slice, slice):
            return Bytes(bytes.__getitem__(self, index_or_slice))
        elif isinstance(index_or_slice, Range):
            return Bytes(bytes.__getitem__(self, index_or_slice.into_slice()))
        else:
            return bytes.__getitem__(self, index_or_slice)
