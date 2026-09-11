; Broad leaf captures precede context-specific roles. Keep this query tied to
; tree-sitter-rock node kinds, not identifier spelling or stdlib names.
(identifier) @variable
(type_identifier) @type
(type_hole) @type
(wildcard_pattern) @variable
(call_hole) @variable
(comment) @comment
(string) @string
(char) @string
[(number) (float)] @number
(boolean) @constant.builtin

[
  "struct" "enum" "trait" "impl" "macro" "infix" "mod" "extern"
  "type" "where" "for" "in" "if" "then" "else" "match" "while"
  "loop" "unsafe" "return" "break" "continue" "mut" "as" "lang"
] @keyword

[(operator) "&" "^" "*" ".." "..="] @operator
"=" @operator.assignment
["->" "!->" "~>" "=>"] @operator.arrow
["(" ")" "[" "]"] @punctuation.bracket
["," ";"] @punctuation.delimiter
":" @punctuation.annotation
["." "::"] @punctuation.access
[(bang_call_suffix) (propagate_suffix)] @punctuation.special
(receiver) @variable.builtin
"@" @variable.builtin

(import_declaration ">" @keyword.import)
(export_declaration "<" @keyword.export)
(struct_field "<" @keyword.export)
(import_path (identifier) @module)
(module_declaration name: (identifier) @module)
(type_path (identifier) @module)
(path_expression (identifier) @module)

(struct_field name: (identifier) @property)
(named_field name: (identifier) @property)
(pattern_field (identifier) @property)
(field_suffix field: [(identifier) (number)] @property)
(field_section_expression field: [(identifier) (number)] @property)
(self_expression (identifier) @property)

(function_declaration name: (_) @function)
(function_signature name: (_) @function)
(extern_signature name: (_) @function)
(parameter_list (identifier_pattern (identifier) @variable.parameter))
(parameter_list (mutable_pattern (identifier) @variable.parameter))
(call_expression function: (identifier) @function)
(call_expression function: (postfix_expression (field_suffix field: (identifier) @function) .))
(call_expression function: (path_expression (identifier) @function .))
(qualified_call_expression function: (path_expression (identifier) @function .))
(postfix_expression value: (identifier) @function . (bang_call_suffix))
(postfix_expression (field_suffix field: (identifier) @function) . (bang_call_suffix))
(postfix_expression value: (path_expression (identifier) @function .) . (bang_call_suffix))
(postfix_expression value: (qualified_expression (identifier) @function .) . (bang_call_suffix))

[(native_operator) (negated_native_operator)] @function.builtin
(macro_declaration name: (identifier) @function.macro)
(macro_invocation name: (identifier) @function.macro)
(macro_sigil) @function.macro
(macro_token "$" @function.macro)
(language_item_marker role: (identifier) @attribute)
