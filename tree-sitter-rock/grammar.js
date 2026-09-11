module.exports = grammar({
  name: 'rock',

  word: $ => $.identifier,

  externals: $ => [
    $._newline,
    $._indent,
    $._call_indent,
    $._dedent,
    $.spaced_dot,
    $.spaced_operator,
    $.stuck_operator,
    $.postfix_continuation,
    $.macro_sigil,
  ],

  extras: $ => [
    /[ \t\f]+/,
    $.comment,
  ],

  supertypes: $ => [
    $.expression,
    $.pattern,
    $.type,
  ],

  conflicts: $ => [
    [$.impl_declaration],
    [$.constructor_generic_parameter],
    [$.macro_block],
    [$.constructor_pattern],
    [$.named_constructor_pattern],
    [$.self_expression],
    [$.constructor_expression],
    [$._type_atom, $._atom],
    [$.macro_invocation],
    [$.unit_type, $.unit_expression],
    [$.type_path, $.path_expression],
    [$.associated_type, $.type_path, $.path_expression],
    [$.expression, $.if_expression],
    [$.expression, $.while_expression],
    [$.expression, $.range_expression],
    [$.type, $.type_application_or_atom],
    [$.associated_type, $.type_path],
    [$.type_arguments],
    [$._atom, $.binding_pattern],
    [$._atom, $.identifier_pattern],
    [$._atom, $.literal_pattern],
    [$._atom, $.rest_pattern],
    [$.identifier_pattern, $.mutable_pattern],
    [$.type_hole, $.wildcard_pattern],
    [$.array_expression, $.array_pattern],
    [$.tuple_expression, $.tuple_pattern],
    [$.call_hole, $.wildcard_pattern],
    [$._atom, $.identifier_pattern, $.mutable_pattern],
    [$.parameter_list, $.pattern_field],
    [$.parameter_list, $.array_pattern],
    [$.parameter_list, $.constructor_pattern],
    [$.call_expression, $.binary_expression],
    [$.block, $.call_expression],
    [$.call_expression, $.postfix_expression],
    [$._statement, $.multiline_argument_block],
    [$._atom, $.path_expression],
    [$.assignment_target, $._atom],
    [$.block],
    [$.assignment_target, $._postfix_expression],
    [$.dereference_postfix_operand, $._atom],
    [$.path_expression],
    [$._type_atom, $.associated_type, $.type_path],
    [$.qualified_expression],
    [$.type_path],
  ],

  rules: {
    program: $ => repeat(choice($._newline, $._top_level_item)),

    _top_level_item: $ => choice(
      $.language_item_marker,
      $.import_declaration,
      $.export_declaration,
      $.function_declaration,
      $.function_signature,
      $.struct_declaration,
      $.enum_declaration,
      $.trait_declaration,
      $.impl_declaration,
      $.macro_declaration,
      $.infix_declaration,
      $.type_alias,
      $.module_declaration,
      $.extern_signature,
    ),

    comment: _ => token(choice(
      seq('//', /[^\r\n]*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'),
    )),

    identifier: _ => /[a-z_][a-zA-Z0-9_]*/,
    type_identifier: _ => /[A-Z][a-zA-Z0-9_]*/,
    number: _ => /[0-9]+/,
    float: _ => /[0-9]+\.[0-9]+/,
    string: _ => /"([^"\\]|\\.)*"/,
    char: _ => /'([^'\\]|\\.)*'/,
    boolean: _ => choice('true', 'false'),
    native_operator: _ => token(prec(2, /~[A-Z][a-zA-Z0-9_]*/)),
    negated_native_operator: _ => token(prec(3, /!~[A-Z][a-zA-Z0-9_]*/)),
    operator: _ => token(prec(-1, /[+\-*\/%=!<>$|&;^~]+/)),
    binary_operator: _ => token(prec(3, choice(
      '==', '!=', '<=', '>=', '&&', '||', '<', '>',
    ))),
    symbolic_operator_name: _ => token(prec(3, /[<>][+\-*\/%=!<>$|&;^~]+/)),

    language_item_marker: $ => seq(
      'lang',
      field('role', $.identifier),
    ),

    import_declaration: $ => seq('>', field('path', $.import_path)),
    export_declaration: $ => seq(
      '<',
      choice(
        field('declaration', $._exportable_declaration),
        field('path', $.import_path),
      ),
    ),

    _exportable_declaration: $ => choice(
      $.function_declaration,
      $.function_signature,
      $.struct_declaration,
      $.enum_declaration,
      $.trait_declaration,
      $.impl_declaration,
      $.type_alias,
      $.module_declaration,
      $.extern_signature,
    ),

    import_path: $ => seq(
      choice($.identifier, $.type_identifier),
      repeat(seq('::', choice($.identifier, $.type_identifier, $.operator))),
      optional(seq('::', '*')),
    ),

    module_declaration: $ => seq('mod', field('name', $.identifier)),

    extern_signature: $ => seq(
      'extern',
      optional('unsafe'),
      field('name', $._function_name),
      ':',
      field('type', $.type),
      optional($.where_clause),
    ),

    infix_declaration: $ => seq(
      'infix',
      field('precedence', $.number),
      field('operator', $.operator),
    ),

    type_alias: $ => seq(
      'type',
      field('name', $.type_identifier),
      optional($.generic_parameters),
      '=',
      field('value', $.type),
    ),

    function_declaration: $ => seq(
      optional('unsafe'),
      optional(field('receiver', $.receiver)),
      field('name', $._function_name),
      '=',
      optional(field('parameters', $.parameter_list)),
      field('arrow', $.function_arrow),
      field('body', choice($.simple_binary_expression, $.block)),
    ),

    function_signature: $ => seq(
      optional('unsafe'),
      optional(field('receiver', $.receiver)),
      field('name', $._function_name),
      ':',
      field('type', $.type),
      optional($.where_clause),
    ),

    _function_name: $ => choice(
      $.identifier,
      $.operator,
      alias($.symbolic_operator_name, $.operator),
    ),
    receiver: _ => choice('@', '^@', '~@'),
    function_arrow: _ => choice('->', '!->', '~>'),

    parameter_list: $ => seq(
      $.pattern,
      repeat(seq(',', $.pattern)),
      optional(','),
    ),

    struct_declaration: $ => seq(
      'struct',
      field('name', $.type_identifier),
      optional($.generic_parameters),
      optional(field('body', $.declaration_block)),
    ),

    struct_field: $ => seq(
      optional(field('export', '<')),
      field('name', $.identifier),
      ':',
      field('type', $.type),
      optional(seq('=', field('default', $.expression))),
    ),

    enum_declaration: $ => seq(
      'enum',
      field('name', $.type_identifier),
      optional($.generic_parameters),
      optional(field('body', $.enum_block)),
    ),

    enum_variant: $ => seq(
      optional(seq($.language_item_marker, repeat1($._newline))),
      field('name', $.type_identifier),
      optional(choice(
        field('fields', $.type_arguments),
        field('fields', $.variant_field_block),
      )),
    ),

    trait_declaration: $ => seq(
      'trait',
      field('name', $.type_identifier),
      optional($.generic_parameters),
      optional(seq('for', field('target', $.generic_parameter))),
      optional($.where_clause),
      optional(field('body', $.trait_block)),
    ),

    impl_declaration: $ => seq(
      'impl',
      optional(field('trait', $.type)),
      optional(seq('for', field('target', $.type))),
      optional($.where_clause),
      optional(field('body', $.impl_block)),
    ),

    associated_type_declaration: $ => seq(
      'type',
      field('name', choice($.type_identifier, $.kinded_generic_parameter)),
    ),

    associated_type_definition: $ => seq(
      'type',
      field('name', choice($.type_identifier, $.kinded_generic_parameter)),
      '=',
      field('value', $.type),
    ),

    declaration_block: $ => seq(
      $._indent,
      repeat($._newline),
      $.struct_field,
      repeat(seq(repeat1($._newline), $.struct_field)),
      repeat($._newline),
      $._dedent,
    ),

    variant_field_block: $ => seq(
      $._indent,
      repeat($._newline),
      $.struct_field,
      repeat(seq(repeat1($._newline), $.struct_field)),
      repeat($._newline),
      $._dedent,
    ),

    enum_block: $ => seq(
      $._indent,
      repeat($._newline),
      $.enum_variant,
      repeat(seq(repeat1($._newline), $.enum_variant)),
      repeat($._newline),
      $._dedent,
    ),

    trait_block: $ => seq(
      $._indent,
      repeat($._newline),
      $._trait_member,
      repeat(seq(repeat1($._newline), $._trait_member)),
      repeat($._newline),
      $._dedent,
    ),

    _trait_member: $ => choice(
      $.language_item_marker,
      $.associated_type_declaration,
      $.function_signature,
      $.function_declaration,
    ),

    impl_block: $ => seq(
      $._indent,
      repeat($._newline),
      $._impl_member,
      repeat(seq(repeat1($._newline), $._impl_member)),
      repeat($._newline),
      $._dedent,
    ),

    _impl_member: $ => choice(
      $.language_item_marker,
      $.associated_type_definition,
      $.function_signature,
      $.function_declaration,
    ),

    generic_parameters: $ => seq(
      $.generic_parameter,
      repeat(seq(',', $.generic_parameter)),
    ),

    generic_parameter: $ => choice(
      field('name', $.type_identifier),
      $.constructor_generic_parameter,
      $.kinded_generic_parameter,
    ),

    constructor_generic_parameter: $ => seq(
      optional('('),
      field('name', $.type_identifier),
      $.type_hole,
      repeat(seq(',', $.type_hole)),
      optional(')'),
    ),

    kinded_generic_parameter: $ => seq(
      '(',
      field('name', $.type_identifier),
      ':',
      field('kind', $.kind),
      ')',
    ),

    kind: $ => prec.right(seq(
      choice('Type', seq('(', $.kind, ')')),
      optional(seq('->', $.kind)),
    )),

    where_clause: $ => seq(
      'where',
      $.where_predicate,
      repeat(seq(',', $.where_predicate)),
    ),

    where_predicate: $ => seq(
      field('subject', $.generic_parameter),
      optional(seq(':', field('bound', $.type))),
    ),

    type: $ => choice(
      $.function_type,
      $.type_application,
      $._type_atom,
    ),

    _type_atom: $ => choice(
      $.type_identifier,
      $.type_path,
      $.type_hole,
      $.unit_type,
      $.tuple_type,
      $.array_type,
      $.reference_type,
      $.pointer_type,
      $.associated_type,
      $.type_lambda,
      $.parenthesized_type,
    ),

    type_hole: _ => '_',
    unit_type: _ => seq('(', ')'),
    parenthesized_type: $ => seq('(', $.type, ')'),

    tuple_type: $ => seq(
      '(',
      $.type,
      ',',
      optional(seq($.type, repeat(seq(',', $.type)), optional(','))),
      ')',
    ),

    array_type: $ => seq(
      '[',
      field('element', $.type),
      optional(seq(';', field('length', $.number))),
      ']',
    ),

    reference_type: $ => seq('&', optional(choice('mut', '^')), $.type),
    pointer_type: $ => prec.right(seq('*', $.type)),

    associated_type: $ => seq(
      field('owner', $.type_identifier),
      '::',
      field('member', $.type_identifier),
    ),

    type_path: $ => seq(
      choice($.identifier, $.type_identifier),
      repeat1(seq('::', $.type_identifier)),
    ),

    type_application: $ => prec.left(5, seq(
      $._type_atom,
      field('arguments', $.type_arguments),
    )),

    type_arguments: $ => seq(
      choice($._type_atom, $.nested_type_argument),
      repeat(seq(',', choice($._type_atom, $.nested_type_argument))),
    ),

    nested_type_argument: $ => prec(6, seq(
      field('constructor', $.type_identifier),
      field('argument', $.type_identifier),
    )),

    function_type: $ => prec.right(1, seq($.type_application_or_atom, '->', $.type)),
    type_application_or_atom: $ => choice($.type_application, $._type_atom),

    type_lambda: $ => prec.right(seq(
      '\\',
      $.generic_parameter,
      repeat(seq(',', $.generic_parameter)),
      '->',
      $.type,
    )),

    block: $ => choice(
      field('body', $._statement),
      seq(
        $._indent,
        repeat($._newline),
        field('body', $._statement),
        repeat(seq(repeat($._newline), field('body', $._statement))),
        repeat($._newline),
        $._dedent,
      ),
    ),

    inline_continuation: $ => choice(
      $.identifier,
      $.type_identifier,
      $.number,
      $.float,
      $.string,
      $.char,
      $.boolean,
      $.native_operator,
      $.operator,
      '(', ')', '[', ']', ',', ':', '::', '.', '..', '?', '!', '@',
    ),

    _statement: $ => choice(
      $.return_expression,
      $.break_expression,
      $.continue_expression,
      $.assignment,
      $.multiline_argument_block,
      $.expression,
    ),

    assignment: $ => prec.right(seq(
      field('left', $.assignment_target),
      optional(seq(':', field('type', $.type))),
      '=',
      field('right', $.expression),
    )),

    assignment_target: $ => choice(
      $.identifier,
      $.mutable_pattern,
      $.tuple_pattern,
      $.array_pattern,
      $.postfix_expression,
      $.unary_assignment_target,
    ),

    unary_assignment_target: $ => seq(
      field('operator', alias($.stuck_operator, $.operator)),
      field('operand', choice(
        $.identifier,
        $.self_expression,
        $.parenthesized_expression,
        $.tuple_expression,
        $.dereference_postfix_operand,
      )),
    ),

    expression: $ => choice(
      $.lambda_expression,
      $.if_expression,
      $.match_expression,
      $.for_expression,
      $.while_expression,
      $.loop_expression,
      $.unsafe_expression,
      $.macro_invocation,
      $.range_expression,
      $.binary_expression,
      $._call_expression,
    ),

    lambda_expression: $ => seq(
      // Prefer `x -> body` over calling x with a parameterless lambda.
      optional(prec.dynamic(40, $.parameter_list)),
      field('arrow', $.function_arrow),
      field('body', $.block),
    ),

    binary_expression: $ => prec.dynamic(20, prec.left(1, seq(
      field('left', $._call_expression),
      field('operator', alias($.spaced_operator, $.operator)),
      field('right', choice($._call_expression, $.indented_binary_operand)),
      repeat(seq(
        field('operator', alias($.spaced_operator, $.operator)),
        field('right', choice($._call_expression, $.indented_binary_operand)),
      )),
    ))),

    range_expression: $ => prec.right(0, seq(
      optional(field('start', choice($.binary_expression, $._call_expression))),
      field('operator', choice('..', '..=')),
      optional(field('end', choice($.binary_expression, $._call_expression))),
    )),

    indented_binary_operand: $ => seq(
      $._call_indent,
      repeat($._newline),
      field('value', $.expression),
      repeat($._newline),
      $._dedent,
    ),

    simple_binary_expression: $ => prec.dynamic(30, prec.left(2, seq(
      field('left', $._postfix_expression),
      field('operator', alias($.binary_operator, $.operator)),
      field('right', $._postfix_expression),
      repeat(seq(
        field('operator', alias($.binary_operator, $.operator)),
        field('right', $._postfix_expression),
      )),
    ))),

    _call_expression: $ => choice(
      $.call_expression,
      $._postfix_expression,
    ),

    call_expression: $ => prec.dynamic(10, prec.left(5, choice(
      seq(
        field('function', $._postfix_expression),
        field('argument', $._call_argument),
        repeat(seq(',', field('argument', $._call_argument))),
      ),
      seq(
        field('function', $._postfix_expression),
        field('arguments', $.multiline_argument_block),
      ),
    ))),

    multiline_argument_block: $ => seq(
      $._call_indent,
      repeat($._newline),
      field('argument', $.expression),
      repeat(seq(repeat($._newline), field('argument', $.expression))),
      repeat($._newline),
      $._dedent,
    ),

    _call_argument: $ => choice(
      $.call_hole,
      $._postfix_expression,
      $.lambda_expression,
    ),
    call_hole: _ => '_',

    _postfix_expression: $ => choice(
      $.postfix_expression,
      $.qualified_call_expression,
      $.unary_expression,
      $.cast_expression,
      $._atom,
    ),

    postfix_expression: $ => prec.left(8, seq(
      field('value', choice($.unary_expression, $.cast_expression, $._atom)),
      repeat1(field('operation', choice(
        $.index_suffix,
        $.field_suffix,
        $.propagate_suffix,
        $.bang_call_suffix,
      ))),
    )),

    index_suffix: $ => seq('[', field('index', $.expression), ']'),
    field_suffix: $ => seq('.', field('field', choice($.identifier, $.number))),
    propagate_suffix: _ => '?',
    bang_call_suffix: _ => '!',

    unary_expression: $ => prec.right(9, seq(
      field('operator', choice(
        seq('&', choice('mut', '^')),
        alias($.stuck_operator, $.operator),
      )),
      field('operand', $._postfix_expression),
    )),

    dereference_expression: $ => prec.right(10, seq(
      token(prec(2, '*')),
      field('operand', choice(
        $.identifier,
        $.self_expression,
        $.parenthesized_expression,
        $.dereference_postfix_operand,
      )),
    )),

    dereference_postfix_operand: $ => prec.left(seq(
      choice($.identifier, $.self_expression, $.parenthesized_expression),
      repeat1(choice(
        $.index_suffix,
        $.field_suffix,
        $.propagate_suffix,
        $.bang_call_suffix,
      )),
    )),

    cast_expression: $ => prec.left(7, seq(
      field('value', choice($.postfix_expression, $.unary_expression, $._atom)),
      'as',
      field('type', $.type),
    )),

    _atom: $ => choice(
      $.identifier,
      $.type_identifier,
      $.path_expression,
      $.self_expression,
      $.postfix_continuation,
      $.native_operator,
      $.number,
      $.float,
      $.string,
      $.char,
      $.boolean,
      $.unit_expression,
      $.tuple_expression,
      $.array_expression,
      $.field_section_expression,
      $.multiplication_section_expression,
      $.typed_parenthesized_expression,
      $.parenthesized_expression,
      $.constructor_expression,
      $.type_qualified_constructor_expression,
      $.qualified_expression,
    ),

    path_expression: $ => seq(
      choice($.identifier, $.type_identifier),
      repeat1(seq('::', choice($.identifier, $.type_identifier))),
    ),

    qualified_call_expression: $ => prec.dynamic(20, prec.right(10, seq(
      field('function', $.path_expression),
      field('argument', $._call_argument),
      repeat(seq(',', field('argument', $._call_argument))),
    ))),

    self_expression: $ => seq('@', optional($.identifier)),
    unit_expression: _ => seq('(', ')'),
    parenthesized_expression: $ => seq(
      '(',
      choice($.dereference_expression, $.expression),
      ')',
    ),

    field_section_expression: $ => seq(
      '(',
      '.',
      field('field', choice($.identifier, $.number)),
      repeat(choice(
        $.field_suffix,
        $.propagate_suffix,
        $.bang_call_suffix,
        $.operator,
        $.identifier,
        $.number,
      )),
      ')',
    ),

    multiplication_section_expression: _ => token(prec(
      100,
      /\(\*[ \t]+[0-9]+[ \t]*\)/,
    )),
    typed_parenthesized_expression: _ => token(prec(
      100,
      /\([^()\r\n]+\):[ \t]*[A-Z][a-zA-Z0-9_]*/,
    )),

    tuple_expression: $ => choice(
      seq(
        '(',
        $.expression,
        ',',
        $.expression,
        repeat(seq(',', $.expression)),
        optional(','),
        ')',
      ),
      seq(
        '(',
        $._indent,
        repeat($._newline),
        $.expression,
        ',',
        repeat(seq(repeat($._newline), $.expression, optional(','))),
        repeat($._newline),
        $._dedent,
        ')',
      ),
    ),

    array_expression: $ => choice(
      seq(
        '[',
        optional(choice(
          seq($.expression, ';', $.number),
          seq($.expression, repeat(seq(',', $.expression)), optional(',')),
        )),
        ']',
      ),
      seq(
        '[',
        $._indent,
        repeat($._newline),
        $.expression,
        optional(','),
        repeat(seq(repeat($._newline), $.expression, optional(','))),
        repeat($._newline),
        $._dedent,
        ']',
      ),
    ),

    constructor_expression: $ => prec(4, choice(
      seq(
        field('constructor', choice($.type_identifier, $.path_expression)),
        choice(
          seq(
            field('field', $.named_field),
            repeat(seq(optional(','), field('field', $.named_field))),
          ),
          field('fields', $.constructor_field_block),
        ),
      ),
      prec(5, seq(
        field('constructor', choice($.type_identifier, $.path_expression)),
        field('generic_argument', $.type_identifier),
        repeat(seq(',', field('generic_argument', $.type_identifier))),
        field('fields', $.constructor_field_block),
      )),
      prec(5, seq(
        field('constructor', $.type_identifier),
        field('argument', $.identifier),
      )),
    )),

    constructor_field_block: $ => seq(
      $._indent,
      repeat($._newline),
      field('field', $.named_field),
      repeat(seq(repeat1($._newline), field('field', $.named_field))),
      repeat($._newline),
      $._dedent,
    ),

    named_field: $ => seq(field('name', $.identifier), ':', field('value', $.expression)),

    qualified_expression: $ => prec.dynamic(50, prec(20, seq(
      '(',
      $.type,
      ')',
      repeat1(seq('::', choice($.identifier, $.type_identifier))),
    ))),

    type_qualified_constructor_expression: $ => prec(100, seq(
      '(',
      field('type', $.type_identifier),
      $.type_hole,
      optional(seq(',', $.type_identifier)),
      ')',
      repeat1(seq('::', field('member', choice($.identifier, $.type_identifier)))),
    )),

    if_expression: $ => prec.right(seq(
      'if',
      field('condition', choice(
        $.negated_native_condition,
        $.assignment_condition,
        $.simple_binary_expression,
        $.expression,
      )),
      optional(choice('then', seq(repeat1($._newline), 'then'))),
      field('consequence', $.block),
      optional(choice(
        seq('else', field('alternative', choice($.if_expression, $.block))),
        seq(
          repeat1($._newline),
          'else',
          field('alternative', choice($.if_expression, $.block)),
        ),
      )),
      repeat($._newline),
    )),

    assignment_condition: $ => seq($.pattern, '=', $.expression),

    negated_native_condition: $ => seq(
      field('function', $.negated_native_operator),
      field('argument', $._call_argument),
      repeat(seq(',', field('argument', $._call_argument))),
    ),

    match_expression: $ => seq(
      'match',
      field('value', $.expression),
      field('body', $.match_block),
    ),

    match_block: $ => seq(
      $._indent,
      repeat($._newline),
      $.match_arm,
      repeat(seq(
        repeat1($._newline),
        $.match_arm,
      )),
      repeat($._newline),
      $._dedent,
    ),

    match_arm: $ => seq(
      field('pattern', $.pattern),
      optional(seq('if', field('guard', $.expression))),
      '=>',
      field('body', $.block),
    ),

    for_expression: $ => seq(
      'for',
      field('pattern', $.pattern),
      'in',
      field('iterator', $.expression),
      field('body', $.block),
    ),

    while_expression: $ => seq(
      'while',
      field('condition', choice(
        $.assignment_condition,
        $.simple_binary_expression,
        $.expression,
      )),
      field('body', $.block),
    ),

    loop_expression: $ => seq('loop', field('body', $.block)),
    unsafe_expression: $ => seq('unsafe', field('body', $.block)),
    return_expression: $ => prec.right(seq('return', optional($.expression))),
    break_expression: $ => prec.right(seq('break', optional($.expression))),
    continue_expression: $ => prec.right(seq('continue', optional($.expression))),

    pattern: $ => choice(
      $.wildcard_pattern,
      $.mutable_pattern,
      $.binding_pattern,
      $.reference_pattern,
      $.tuple_pattern,
      $.array_pattern,
      $.named_constructor_pattern,
      $.constructor_pattern,
      $.literal_pattern,
      $.identifier_pattern,
    ),

    wildcard_pattern: _ => '_',
    identifier_pattern: $ => $.identifier,
    mutable_pattern: $ => seq(choice('mut', '^'), $.identifier),
    binding_pattern: $ => prec.right(seq($.identifier, '@', $.pattern)),
    reference_pattern: $ => seq('&', optional(choice('mut', '^')), $.pattern),
    tuple_pattern: $ => seq('(', $.pattern, repeat1(seq(',', $.pattern)), optional(','), ')'),
    array_pattern: $ => seq(
      '[',
      optional(seq(
        choice($.pattern, $.rest_pattern),
        repeat(seq(',', choice($.pattern, $.rest_pattern))),
        optional(','),
      )),
      ']',
    ),
    rest_pattern: $ => seq('..', choice($.identifier, $.mutable_pattern)),
    constructor_pattern: $ => prec(2, seq(
      field('constructor', choice($.type_identifier, $.path_expression)),
      optional(seq(
        field('argument', $.pattern),
        repeat(seq(',', field('argument', $.pattern))),
      )),
    )),
    named_constructor_pattern: $ => prec(3, seq(
      field('constructor', choice($.type_identifier, $.path_expression)),
      field('field', $.pattern_field),
      repeat(seq(',', field('field', $.pattern_field))),
    )),
    pattern_field: $ => seq($.identifier, ':', $.pattern),
    literal_pattern: $ => choice($.number, $.float, $.string, $.char, $.boolean),

    macro_declaration: $ => seq(
      'macro',
      field('name', $.identifier),
      optional($.macro_block),
    ),

    macro_block: $ => seq(
      $._indent,
      repeat($._newline),
      repeat1(choice($._newline, $.macro_token)),
      $._dedent,
    ),

    macro_invocation: $ => seq($.macro_sigil, field('name', $.identifier), repeat($.macro_token)),
    macro_token: $ => choice(
      $.identifier,
      $.type_identifier,
      $.number,
      $.string,
      $.char,
      $.operator,
      ',', ':', '(', ')', '[', ']', '$', '.', '?', '@', '\\',
    ),

  },
});
