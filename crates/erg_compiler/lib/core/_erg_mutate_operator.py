def mutate_operator(x):
    if hasattr(x, "mutate"):
        return x.mutate()
    else:
        return x
