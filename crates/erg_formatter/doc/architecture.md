# Architecture

Erg formatter (hereafter `ergfmt`) goes through two major formatting phases: the transform phase and the emit phase.

The transform phase performs the formatting that is done by transforming the AST, such as reordering imports. Specifically, the following operations are performed:

* Alignment of imports
* Alignment of record fields

In the Emit phase, the formatted AST is output according to style rules. In this phase, for example, import lines that are too long are broken.

* Long container line breaks (including arguments)
* 2 line breaks between function definitions
* Remove unnecessary parts (e.g. spaces, line breaks, parentheses, etc.)
