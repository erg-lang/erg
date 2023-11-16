import erg_compiler
erg_parser = erg_compiler.erg_parser
erg_ast = erg_compiler.erg_parser.ast

module = erg_parser.parse(".i = 1")
d = module.pop()
d.sig = erg_ast.VarSignature.new(erg_ast.Identifier.public("j"), None)
module.push(d)
ast = erg_ast.AST.new("test", module)
code = erg_compiler.compile_ast(ast)
exec(code)
assert j == 1
