class Bytes(bytes):
    def try_new(*b):  # -> Result[Nat]
        return Bytes(bytes(*b))
