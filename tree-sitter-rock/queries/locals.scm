((function_declaration) @local.scope
  (#set! local.scope-inherits false))
[(lambda_expression) (block) (match_arm) (for_expression)] @local.scope

(identifier_pattern (identifier) @local.definition)
(mutable_pattern (identifier) @local.definition)
(binding_pattern (identifier) @local.definition)
(rest_pattern (identifier) @local.definition)

; Expression identifiers exclude member names, paths, and declaration labels.
(expression/identifier) @local.reference
(call_expression function: (identifier) @local.reference)
(call_expression argument: (identifier) @local.reference)
(postfix_expression value: (identifier) @local.reference)
(unary_expression operand: (identifier) @local.reference)
(unary_assignment_target operand: (identifier) @local.reference)
(dereference_expression operand: (identifier) @local.reference)
(dereference_postfix_operand (identifier) @local.reference)
(simple_binary_expression left: (identifier) @local.reference)
(simple_binary_expression right: (identifier) @local.reference)
(binary_expression left: (identifier) @local.reference)
(binary_expression right: (identifier) @local.reference)
(qualified_call_expression argument: (identifier) @local.reference)
(negated_native_condition argument: (identifier) @local.reference)
(constructor_expression argument: (identifier) @local.reference)
(range_expression start: (identifier) @local.reference)
(range_expression end: (identifier) @local.reference)
(cast_expression value: (identifier) @local.reference)
; Plain assignment can update an existing argument; it is not a new binder.
(assignment_target (identifier) @local.reference)
