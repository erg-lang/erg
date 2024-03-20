from abc import ABC, abstractmethod

from _erg_float import Float

class Eq(ABC):
    @abstractmethod
    def __eq__(self, other): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Eq:
            if any("__eq__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Ord(ABC):
    @abstractmethod
    def __lt__(self, other): pass
    def __gt__(self, other): pass
    def __le__(self, other): pass
    def __ge__(self, other): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Ord:
            # TODO: adhoc
            if C == float or C == Float:
                return False
            if any("__lt__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Hash(ABC):
    @abstractmethod
    def __hash__(self): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Hash:
            if any("__hash__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Sized(ABC):
    @abstractmethod
    def __len__(self): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Sized:
            if any("__len__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Add(ABC):
    Output: type

    @abstractmethod
    def __add__(self, other): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Add:
            if any("__add__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Sub(ABC):
    Output: type

    @abstractmethod
    def __sub__(self, other): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Sub:
            if any("__sub__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Mul(ABC):
    Output: type

    @abstractmethod
    def __mul__(self, other): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Mul:
            if any("__mul__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Div(ABC):
    Output: type

    @abstractmethod
    def __truediv__(self, other): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Div:
            if any("__truediv__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Pos(ABC):
    Output: type

    @abstractmethod
    def __pos__(self): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Pos:
            if any("__pos__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented

class Neg(ABC):
    Output: type

    @abstractmethod
    def __neg__(self): pass

    @classmethod
    def __subclasshook__(cls, C):
        if cls is Neg:
            if any("__neg__" in B.__dict__ for B in C.__mro__):
                return True
        return NotImplemented
