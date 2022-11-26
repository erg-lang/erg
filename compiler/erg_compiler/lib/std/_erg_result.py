from typing import TypeVar, Union, _SpecialForm, _type_check

class Error:
    def __init__(self, message):
        self.message = message

T = TypeVar("T")
@_SpecialForm
def Result(self, parameters):
    """Result type.

    Result[T] is equivalent to Union[T, Error].
    """
    arg = _type_check(parameters, f"{self} requires a single type.")
    return Union[arg, Error]

def is_ok(obj: Result[T]) -> bool:
    return not isinstance(obj, Error)
