# HACK: import MutType to suppress segfault in CPython 3.10 (cause unknown)
from _erg_list import List, UnsizedList
from _erg_bool import Bool
from _erg_bytes import Bytes
from _erg_contains_operator import contains_operator
from _erg_dict import Dict
from _erg_float import Float, FloatMut
from _erg_int import Int, IntMut
from _erg_mutate_operator import mutate_operator
from _erg_nat import Nat, NatMut
from _erg_range import (ClosedRange, LeftOpenRange, OpenRange, Range,
                        RangeIterator, RightOpenRange)
from _erg_result import Error, is_ok
from _erg_ratio import Ratio, RatioMut
from _erg_set import Set
from _erg_str import Str, StrMut
from _erg_type import MutType as _MutType
from _erg_iterable import (iterable_map, iterable_filter, iterable_reduce,
                        iterable_nth, iterable_skip, iterable_all,
                        iterable_any, iterable_position, iterable_find,
                        iterable_chain)

Record = tuple


class Never:
    pass
