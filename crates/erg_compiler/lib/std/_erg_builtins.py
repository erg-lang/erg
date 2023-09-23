# in Python 3.7, `sum` takes no keyword arguments
def sum(iterable, start=0):
    s = start
    for i in iterable:
        s += i
    return s
