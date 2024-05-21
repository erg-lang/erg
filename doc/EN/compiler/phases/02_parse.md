# Parsing

The `Parser` defined in `erg_parser/parse.rs` performs the parsing. It is also a disposable structure, primarily used wrapped in `ParserRunner`.
The `Parser` performs recursive descent parsing. To avoid stack overflow, on Windows where the default stack size is small, it is executed on a separate thread with a manually specified stack size.

A distinctive feature of Erg's syntax is that it is case-sensitive, and in the worst case, the syntax cannot be determined no matter how much it is pre-read.

For example, consider the following syntax.

```python
a, b, c, d, e, (...)
```

If an '=' appears within (...), it is determined to be a tuple destructuring assignment. If it does not appear, it is simply a tuple.
However, there is no upper limit to the number of tokens needed to determine which it is.

Therefore, in such cases, the `Parser` first assumes it is a tuple and proceeds with parsing.
If an '=', '->', or '=>' appears before a newline, it is determined to be a destructuring assignment, and the previously parsed tuple is converted to a left-hand value.
Other patterns, such as function definitions, are also parsed in a similar manner.
This is possible because for every left-hand value, there exists a syntactically dual right-hand value (although not every right-hand value has a dual left-hand value).
