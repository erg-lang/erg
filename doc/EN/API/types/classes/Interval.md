# Interval begin, end := WellOrder

A type that represents a subtype of the well-ordered set type (WellOrder). The Interval type has derived types such as PreOpen(x<..y).

```python
Months = 1..12
Alphabet = "a".."z"
Weekdays = Monday..Friday
Winter = November..December or January..February
```

```python
0..1 # integer range
0.0..1.0 # real (rational) range
# or same for 0/1..1/1
```

Computers can't handle numbers with infinite digits, so the range of real numbers is actually the range of rational numbers.