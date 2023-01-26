import os
import glob

"""
Align file prefixes when they are not numbered consecutively.
existing files: 01_foo.md, 03_bar.md, 04_baz.md
result: 01_foo.md, 02_bar.md, 03_baz.md
"""
if __name__ == '__main__':
    prev = None
    diff = None
    for f in sorted(glob.glob("[0-9][0-9]_*")):
        if prev != None:
            now_file_no = int(f.split("_")[0])
            diff = now_file_no - prev
            if diff != 1:
                replace_to = "_".join([f"{now_file_no-diff+1:02d}", *f.split("_")[1:]])
                os.rename(f, replace_to)
                prev = now_file_no - diff + 1
            else:
                prev = now_file_no
        else:
            prev = int(f.split("_")[0])
