; Rock language syntax highlighting - minimal working version

; Declarations - highlight keywords in declarations
(struct_declaration
  "struct" @keyword)
(enum_declaration
  "enum" @keyword)
(trait_declaration
  "trait" @keyword)
(impl_declaration
  "impl" @keyword)
(macro_declaration
  "macro" @keyword)
(infix_declaration
  "infix" @keyword)
(module_declaration
  "mod" @keyword)
(extern_declaration
  "extern" @keyword)
(type_declaration
  "type" @keyword)

; Expressions with keywords
(if_expression) @conditional
(for_expression) @repeat
(while_expression) @repeat
(loop_expression) @repeat
(match_expression) @keyword
(unsafe_expression) @keyword.coroutine

; Control flow
(return_expression) @keyword.return
(break_expression) @keyword
(continue_expression) @keyword

; Visibility
"pub" @keyword
"for" @keyword

; Operators (multi-character only)
(operator) @operator
"=" @operator
"->" @operator
"=>" @operator
"::" @operator

; Literals
(number) @number
(float) @float
(string) @string
(char) @character
(boolean) @boolean
(unit) @constant.builtin

; Identifiers and types
(identifier) @variable
(type_identifier) @type

; Functions
(function_declaration
  (identifier) @function)

(macro_declaration
  (identifier) @function.macro)

(infix_declaration
  (identifier) @function)

; Function and method calls
(call_expression
  (primary_expression
    (identifier) @function.call))

(field_expression
  (identifier) @method.call)

; Lambda parameters
(lambda_expression
  (identifier) @variable.parameter
  "->" @operator)

; Types
(base_type) @type.builtin
(function_type) @type
(array_type) @type
(tuple_type) @type
(reference_type) @type
(pointer_type) @type

; Type definitions
(struct_declaration
  (type_identifier) @type.definition)

(enum_declaration
  (type_identifier) @type.definition)

(trait_declaration
  (type_identifier) @type.definition)

(type_declaration
  (type_identifier) @type.definition)

; Macros
(macro_invocation
  "%" @macro
  (identifier) @function)

; Struct instantiation
(struct_expression
  (type_identifier) @constructor)

; Arrays
(array_expression) @constructor

; Match expressions
(match_expression
  "=>" @operator)

; Pattern matching
(pattern
  (identifier) @variable)
(pattern
  "_") @variable.builtin

; Comments
(line_comment) @comment
(block_comment) @comment
