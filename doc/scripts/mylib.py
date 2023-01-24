import os
import re


def get_file_dirname():
    return os.path.dirname(__file__)


path_join = os.path.join


def str_slice(str: str, first_index: int):
    return str[first_index:]


def eliminate_none_on_match(matched: re.Match):
    class _a:
        def __init__(self):  # コンストラクタ
            self.matched = matched
            self.is_none = True if matched is None else False

    return _a()
