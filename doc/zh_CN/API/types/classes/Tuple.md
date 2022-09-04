# Tuple T: ...Type

A collection that holds objects of multiple types.

## methods

* zip self, other

     Composites two ordered collections (arrays or tuples).

     ``` erg
     assert ([1, 2, 3].zip [4, 5, 6])[0] == (1, 4)
     ```

* zip_by self, op, other

     A method that generalizes zip. You can specify a binary operation to compose.
     `()`, `[]`, `{}`, `{:}` can also be specified as operators, and generate tuples, arrays, sets, and dicts respectively.

     ``` erg
     assert ([1, 2, 3].zip([4, 5, 6]))[0] == (1, 4)
     assert ([1, 2, 3].zip_by((), [4, 5, 6]))[0] == (1, 4)
     assert ([1, 2, 3].zip_by([], [4, 5, 6]))[0] == [1, 4]
     assert ([1, 2, 3].zip_by({}, [4, 5, 6]))[0] == {1, 4}
     assert ([1, 2, 3].zip_by({:}, [4, 5, 6]))[0] == {1: 4}
     assert ([1, 2, 3].zip_by(`_+_`, [4, 5, 6]))[0] == 5
     ```