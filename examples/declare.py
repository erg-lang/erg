x = 0


def f(x: int) -> int:
    return x + 1


class C:
    def __init__(self, x: int) -> None:
        self.x = x

    def f(self, y: int) -> int:
        return self.x + y
