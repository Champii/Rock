; Scope and local variable tracking for Rock

; Function parameters
(lambda_expression
  (identifier) @definition.parameter)

; Function declarations
(function_declaration
  (identifier) @definition.function)

; Struct declarations
(struct_declaration
  (type_identifier) @definition.type)

; Enum declarations
(enum_declaration
  (type_identifier) @definition.type)

; Trait declarations
(trait_declaration
  (type_identifier) @definition.type)

; Macro declarations
(macro_declaration
  (identifier) @definition.macro)

; Pattern bindings in match
(match_expression
  (pattern) @definition.var)

; For loop variables
(for_expression
  (identifier) @definition.var)

; References to variables
(identifier) @reference
(type_identifier) @reference.type

; Function calls
(call_expression
  (primary_expression
    (identifier) @reference.call))
