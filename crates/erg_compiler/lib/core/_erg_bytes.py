from _erg_result import Error
from _erg_range import Range


class Bytes(bytes):
    def try_new(b):  # -> Result[Nat]
        if isinstance(b, bytes):
            return Bytes(bytes(b))
        else:
            return Error("not a bytes")

    def __getitem__(self, index_or_slice):
        if isinstance(index_or_slice, slice):
            return Bytes(bytes.__getitem__(self, index_or_slice))
        elif isinstance(index_or_slice, Range):
            return Bytes(bytes.__getitem__(self, index_or_slice.into_slice()))
        else:
            return bytes.__getitem__(self, index_or_slice)
