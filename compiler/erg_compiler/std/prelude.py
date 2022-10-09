def in_operator(x, y):
    if type(y) == type:
        if isinstance(x, y):
            return True
        # TODO: trait check
        return False
    else:
        return x in y
