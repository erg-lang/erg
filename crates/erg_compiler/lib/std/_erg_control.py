def if__(cond, then, else_=lambda: None):
    if cond:
        return then()
    else:
        return else_()


def for__(iterable, body):
    for i in iterable:
        body(i)


def while__(cond_block, body):
    while cond_block():
        body()


def with__(obj, body):
    obj.__enter__()
    body(obj)
    obj.__exit__()


def discard__(obj):
    pass

def assert__(test, msg=None):
    assert test, msg

def then__(x, f):
    if x == None or x == NotImplemented:
        return x
    else:
        return f(x)
