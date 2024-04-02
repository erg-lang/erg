def iterable_map(iterable, f):
    return map(f, iterable)

def iterable_filter(iterable, f):
    return filter(f, iterable)

def iterable_reduce(iterable, initial, f):
    from functools import reduce
    return reduce(f, iterable, initial)

def iterable_nth(iterable, n):
    from itertools import islice
    return next(islice(iterable, n, None))

def iterable_skip(iterable, n):
    from itertools import islice
    return islice(iterable, n, None)

def iterable_all(iterable, f):
    return all(map(f, iterable))

def iterable_any(iterable, f):
    return any(map(f, iterable))

def iterable_position(iterable, f):
    for i, x in enumerate(iterable):
        if f(x):
            return i
    return None

def iterable_find(iterable, f):
    for x in iterable:
        if f(x):
            return x
    return None

def iterable_chain(*iterables):
    from itertools import chain
    return chain(*iterables)
