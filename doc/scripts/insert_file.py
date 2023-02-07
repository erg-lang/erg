import os
import sys
import glob

"""
Insert a file into files followed by a sequential number. Existing files are shifted by one.
example:
    existing files: 01_foo.md, 02_bar.md, 03_baz.md
    1st arg(inserting file): qux.md
    2nd arg(file no): 2
    result: 01_foo.md, 02_qux.md, 03_bar.md, 04_baz.md
"""
if __name__ == '__main__':
    file = sys.argv[1]
    file_no = sys.argv[2]
    if not file_no.isdigit():
        raise ValueError('File number must be a number')
    else:
        file_no = int(file_no)

    if len(glob.glob("_[0-9][0-9]_*")) > 0:
        raise Exception("Escaped file already exists, rename it")
    # escaping
    for esc in sorted(glob.glob("[0-9][0-9]_*")):
        if int(esc.split("_")[0]) < file_no: continue
        else: os.rename(esc, "_" + esc)

    target = f"{file_no:02d}_" + file
    if os.path.exists(target):
        raise OSError(f"File {target} already exists")
    os.rename(file, target)

    while True:
        nxt = glob.glob(f"_{file_no:02d}_*")
        if len(nxt) == 0:
            exit(0)
        elif len(nxt) >= 2:
            raise ValueError("More than one file with the same number")
        else:
            target = nxt[0]
            replace_to = "_".join([f"{file_no+1:02d}", *target.split("_")[2:]])
            os.rename(target, replace_to)
        file_no += 1
