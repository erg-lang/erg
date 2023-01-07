# This script automatically generates a table of contents (SUMMARY.md) from markdown file titles
# TODO: rewrite in Erg

import os
import re

SUMMARY_TITLE = "SUMMARY_TITLE"
SUMMARY_DESCRIPTION = "SUMMARY_DESCRIPTION"
SUMMARY_MD = "SUMMARY.md"

LANGUAGE_SPECIFIC = {
    "EN": {
        SUMMARY_TITLE: "Summary",
        SUMMARY_DESCRIPTION: """\
This file is generated automatically. If you want to edit this, edit `doc/sync_to_summary.py`
This file is for generating The Erg Book. Do not add badges, etc.\
""",
    },
    "JA": {
        SUMMARY_TITLE: "概要",
        SUMMARY_DESCRIPTION: """\
このファイルは自動生成されます。これを編集したい場合は`doc/sync_to_summary.py`を編集してください。
このファイルはThe Erg Bookを生成するためのものです。バッジなどは付けないでください。\
""",
    },
    "zh_CN": {
        SUMMARY_TITLE: "Summary",  # TODO: translate
        SUMMARY_DESCRIPTION: """\
This file is generated automatically. If you want to edit this, edit `doc/syn_to_summary.py`
This file is for generating The Erg Book. Do not add badges, etc.\
""",  # TODO: translate
    },
    "zh_TW": {
        SUMMARY_TITLE: "Summary",  # TODO: translate
        SUMMARY_DESCRIPTION: """\
This file is generated automatically. If you want to edit this, edit `doc/sync_to_summary.py`
This file is for generating The Erg Book. Do not add badges, etc.\
""",  # TODO: translate
    },
}

title_pattern = re.compile(r"^#\s+(.+)\s*$")
dir_file_name_pattern = re.compile(r"(?:\d+_)?(.+)\.md")


def get_title(file_path):
    with open(file_path, encoding="utf-8") as f:
        for line in f:
            title_match = title_pattern.match(line)
            if title_match is not None:
                return title_match.group(1)
    matched_dir_name = dir_file_name_pattern.match(os.path.basename(file_path))
    if matched_dir_name is None:
        return None
    return matched_dir_name.group(1)


def get_summary(
    base_path: str, dir_relative_path: str, depth: int, current_text: str
) -> str:
    path = f"{base_path}{dir_relative_path}"
    dir_list = sorted(os.listdir(path))
    file_names = [
        f
        for f in dir_list
        if os.path.isfile(os.path.join(path, f)) and (depth != 0 or f != SUMMARY_MD)
    ]
    dir_names = [f for f in dir_list if os.path.isdir(os.path.join(path, f))]
    for file_name in file_names:
        current_text += f"{'  '*depth}- [{get_title(os.path.join(path, file_name))}](.{dir_relative_path}/{file_name})\n"
        dir_file_name_match = dir_file_name_pattern.match(file_name)
        matched_dir_name = None
        if dir_file_name_match is not None:
            matched_dir_name = dir_file_name_match.group(1)
        if matched_dir_name is not None and matched_dir_name in dir_names:
            current_text = get_summary(
                base_path,
                f"{dir_relative_path}/{matched_dir_name}",
                depth + 1,
                current_text,
            )
    return current_text


def main():
    os.chdir(os.path.dirname(__file__))

    for language in LANGUAGE_SPECIFIC.keys():
        syntax_base_path = f"./{language}/syntax"
        with open(
            os.path.join(syntax_base_path, SUMMARY_MD), mode="w", encoding="utf-8"
        ) as f:
            f.write(
                get_summary(
                    syntax_base_path,
                    "",
                    0,
                    f"""\
# {LANGUAGE_SPECIFIC[language][SUMMARY_TITLE]}

<!--
{LANGUAGE_SPECIFIC[language][SUMMARY_DESCRIPTION]}
-->

""",
                )
            )


if __name__ == "__main__":
    main()
