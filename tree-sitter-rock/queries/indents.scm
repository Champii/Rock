; Indentation for Rock

; Function bodies
(function_declaration
  (expression
    (lambda_expression
      (expression) @indent)))

; Struct fields
(struct_declaration
  (identifier) @indent)

; Enum variants
(enum_declaration
  (enum_variant) @indent)

; Trait methods
(trait_declaration
  (trait_member) @indent)

; Impl blocks
(impl_declaration
  (identifier) @indent)

; If expressions
(if_expression
  (_)
  (expression) @indent)

(if_expression
  (_)
  (_)
  (expression) @indent)

; Match arms
(match_expression
  (pattern)
  (expression) @indent)

; For loops
(for_expression
  (_)
  (_)
  (expression) @indent)

; While loops
(while_expression
  (_)
  (expression) @indent)

; Loop expressions
(loop_expression
  (expression) @indent)

; Unsafe blocks
(unsafe_expression
  (expression) @indent)
