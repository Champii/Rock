(comment) @comment

[
  "struct"
  "enum"
  "trait"
  "impl"
  "macro"
  "infix"
  "mod"
  "extern"
  "type"
  "where"
  "for"
  "in"
  "if"
  "then"
  "else"
  "match"
  "while"
  "loop"
  "unsafe"
  "return"
  "break"
  "continue"
  "mut"
  "as"
  "lang"
] @keyword

[(boolean)] @boolean
(number) @number
(float) @number.float
(string) @string
(char) @character
(native_operator) @function.builtin
(operator) @operator
(function_arrow) @operator
(receiver) @variable.builtin
(type_identifier) @type
(type_hole) @type.builtin

(function_declaration name: (identifier) @function)
(function_signature name: (identifier) @function)
(extern_signature name: (identifier) @function)
(macro_declaration name: (identifier) @function.macro)
(macro_invocation name: (identifier) @function.macro)
(struct_declaration name: (type_identifier) @type.definition)
(enum_declaration name: (type_identifier) @type.definition)
(trait_declaration name: (type_identifier) @type.definition)
(type_alias name: (type_identifier) @type.definition)
(struct_field name: (identifier) @property)
(named_field name: (identifier) @property)
(field_suffix field: (identifier) @property)
(language_item_marker role: (identifier) @attribute)

(identifier) @variable
