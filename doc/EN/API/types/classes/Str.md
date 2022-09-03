# Str

(Invariant length) A type that represents a string. The simple `Str` type is the `StrWithLen N` type with the number of characters removed (`Str = StrWithLen _`).

## methods

*isnumeric

Returns whether the string is an Arabic numeral. Use `isunicodenumeric` to judge kanji numerals and other characters that represent numbers (note that this behavior is different from Python).