import os
import re


def get_file_dirname():
    return os.path.dirname(__file__)


path_join = os.path.join


def str_slice(str: str, first_index: int):
    return str[first_index:]
