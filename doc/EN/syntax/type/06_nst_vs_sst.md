# Nominal Subtyping vs. Structural Subtyping

```python
Months = 0..12

# NST
MonthsClass = Class Months
MonthsClass.
    name self =
        match self:
            1 -> "january"
            2 -> "february"
            3 -> "march"
            ...

# SST
MonthsImpl = Patch Months
MonthsImpl.
    name self =
        match self:
            1 -> "January"
            2 -> "February"
            3 -> "March"
            ...

assert 12 in Months
assert 2.name() == "February"
assert not 12 in MonthsClass
assert MonthsClass.new(12) in MonthsClass
# It can use structural types, even though wrapped in a class.
assert MonthsClass.new(12) in Months
# If both exist, class methods take priority.
assert MonthsClass.new(2).name() == "february"
```

## In The End, Which Should I Use, NST or SST?

If you cannot decide which one to use, our recommendation is NST.
SST requires abstraction skills to write code that does not break down in any use case. Good abstraction can lead to high productivity, but wrong abstraction (commonality by appearances) can lead to counterproductive results. (NSTs can reduce this risk by deliberately keeping abstraction to a minimum. If you are not a library implementor, it is not a bad idea to code only with NSTs.

<p align='center'>
    <a href='./05_inheritance.md'>Previous</a> | <a href='./07_patch.md'>Next</a>
</p>
