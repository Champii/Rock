[
  (function_declaration)
  (lambda_expression)
  (match_arm)
  (for_expression)
] @local.scope

(function_declaration name: (identifier) @local.definition)
(function_signature name: (identifier) @local.definition)
(macro_declaration name: (identifier) @local.definition)
(struct_declaration name: (type_identifier) @local.definition.type)
(enum_declaration name: (type_identifier) @local.definition.type)
(trait_declaration name: (type_identifier) @local.definition.type)
(identifier_pattern (identifier) @local.definition)
(mutable_pattern (identifier) @local.definition)

(identifier) @local.reference
(type_identifier) @local.reference.type
