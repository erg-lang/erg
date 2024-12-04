import os


def get_file_dirname():
    return os.path.dirname(__file__)


path_join = os.path.join

path_relpath = os.path.relpath

path_sep = os.path.sep


def str_slice(str: str, first_index: int):
    return str[first_index:]
