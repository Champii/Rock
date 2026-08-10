module.exports = grammar({
  name: 'rock',

  extras: $ => [
    /\s/,
    $.line_comment,
    $.block_comment,
  ],

  conflicts: $ => [
    [$.impl_declaration],
    [$.trait_declaration],
    [$.struct_declaration],
    [$.enum_declaration],
    [$.macro_declaration],
    [$.extern_declaration],
    [$.macro_invocation, $.index_expression],
    [$.return_expression],
    [$.call_expression],
    [$.index_expression],
    [$._call_argument, $.index_expression],
    [$._statement, $.index_expression],
    [$.expression, $.call_expression],
    [$.struct_expression],
    [$.lambda_expression, $.index_expression],
    [$.primary_expression, $.lambda_expression],
    [$.macro_invocation],
    [$.index_expression, $.unary_expression],
    [$.match_expression],
    [$.index_expression, $.match_expression],
    [$.index_expression, $.loop_expression],
    [$.index_expression, $.while_expression],
    [$.index_expression, $.for_expression],
    [$.index_expression, $.unsafe_expression],
    [$.index_expression, $.return_expression],
    [$.call_expression, $.index_expression],
    [$.function_declaration, $.assignment],
    [$.function_declaration, $.assignment, $.index_expression],
    [$.primary_expression, $.struct_expression],
    [$.struct_expression, $.index_expression],
    [$.function_type, $.reference_type],
    [$.function_type, $.pointer_type],
    [$.function_declaration, $.index_expression],
    [$.function_type, $.enum_declaration],
    [$.function_type, $.impl_declaration],
    [$.macro_declaration, $.index_expression],
    [$.function_type, $.type_declaration],
    [$.index_expression, $.if_expression],
    [$.primary_expression, $.pattern],
    [$.function_type, $.struct_declaration],
    [$.array_expression, $.pattern],
    [$.lambda_expression, $.pattern],
    [$.primary_expression, $.lambda_expression, $.pattern],
    [$.array_expression, $.index_expression],
    [$.function_type],
    [$.impl_declaration, $.index_expression],
    [$.struct_declaration, $.index_expression],
    [$.if_expression],
    [$.enum_variant],
    [$.function_type, $.enum_variant],
    [$.function_type, $.trait_member],
    [$.trait_member, $.index_expression],
    [$.type_application, $.function_type],
    [$.type_application, $.type_lambda],
    [$.type_application, $.tuple_type],
    [$.function_type, $.type_lambda],
    [$.generic_declaration, $.type_application],
    [$.type_qualified_expression, $.parenthesized_expression],
    [$._type_atom, $.primary_expression],
    [$._type, $._type_atom],
    [$.function_type, $.function_declaration],
    [$._type, $.primary_expression],
    [$._generic_declaration_body],
    [$._type, $._generic_declaration_body],
    [$._type, $._type_atom, $._generic_declaration_body],
    [$._type_atom, $._generic_declaration_body],
    [$.function_type, $.trait_declaration],
    [$.function_type, $.where_predicate],
    [$._type_atom, $.pattern],
    [$.type_hole, $.pattern],
    [$._type, $._type_atom, $.pattern],
    [$._type, $.primary_expression, $.pattern],
    [$._type_atom, $.primary_expression, $.pattern],
  ],

  rules: {
    program: $ => repeat($._statement),

    _statement: $ => choice(
      $.function_declaration,
      $.struct_declaration,
      $.enum_declaration,
      $.trait_declaration,
      $.impl_declaration,
      $.macro_declaration,
      $.infix_declaration,
      $.type_declaration,
      $.module_declaration,
      $.extern_declaration,
      $.assignment,
      $.expression,
    ),

    // Comments
    line_comment: $ => token(seq('//', /.*/)),
    block_comment: $ => token(seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')),

    // Identifiers and Literals
    identifier: $ => /[a-z_][a-zA-Z0-9_]*/,
    type_identifier: $ => /[A-Z][a-zA-Z0-9_]*/,
    number: $ => /\d+/,
    float: $ => /\d+\.\d+/,
    string: $ => /"([^"\\]|\\.)*"/,
    char: $ => /'([^'\\]|\\.)*'/,
    boolean: $ => choice('true', 'false'),
    unit: $ => '()',

    // Keywords
    keyword: $ => choice(
      'struct', 'enum', 'trait', 'impl', 'if', 'then', 'else',
      'for', 'in', 'while', 'loop', 'macro', 'true', 'false',
      'return', 'continue', 'break', 'infix', 'mod', 'extern',
      'match', 'unsafe', 'type', 'mut', 'pub', 'where'
    ),

    // Types
    base_type: $ => choice(
      'Int8', 'Int16', 'Int32', 'Int64',
      'UInt8', 'UInt16', 'UInt32', 'UInt64',
      'Float32', 'Float64',
      'Bool', 'String', 'Char'
    ),

    _type: $ => choice(
      $.base_type,
      $.type_identifier,
      $.type_lambda,
      $.function_type,
      $.type_application,
      $.array_type,
      $.tuple_type,
      $.reference_type,
      $.pointer_type,
    ),

    _type_atom: $ => choice(
      $.base_type,
      $.type_identifier,
      $.type_hole,
      $.array_type,
      $.tuple_type,
      $.reference_type,
      $.pointer_type,
    ),

    function_type: $ => prec.right(seq(
      $._type,
      '->',
      $._type
    )),

    type_application: $ => prec.left(7, seq(
      choice($.type_application, $._type_atom),
      choice(
        $._type_atom,
        seq(',', $._type_atom),
      ),
    )),

    type_lambda: $ => prec.right(1, seq(
      '\\',
      $._type_lambda_parameter,
      repeat(seq(',', $._type_lambda_parameter)),
      '->',
      $._type,
    )),

    _type_lambda_parameter: $ => choice(
      $.type_identifier,
      seq('(', $.generic_declaration, ')'),
    ),

    type_hole: $ => '_',

    array_type: $ => seq('[', $._type, optional(seq(';', $.number)), ']'),
    tuple_type: $ => prec(8, seq('(', optional(sepBy(',', $._type)), ')')),
    reference_type: $ => choice(seq('&mut', $._type), seq('&', $._type)),
    pointer_type: $ => seq('*', $._type),

    generic_declaration: $ => choice(
      seq(
        '(',
        field('name', $.type_identifier),
        ':',
        $.kind,
        ')',
      ),
      seq('(', $._generic_declaration_body, ')'),
      $._generic_declaration_body,
    ),

    _generic_declaration_body: $ => seq(
      field('name', $.type_identifier),
      optional(seq(
        $.type_hole,
        repeat(seq(',', $.type_hole)),
      )),
    ),

    kind: $ => prec.right(1, seq(
      $._kind_atom,
      optional(seq('->', $.kind)),
    )),

    _kind_atom: $ => choice(
      'Type',
      seq('(', $.kind, ')'),
    ),

    generic_declarations: $ => seq(
      $.generic_declaration,
      repeat(seq(',', $.generic_declaration)),
    ),

    where_clause: $ => seq('where', $.where_predicate),

    where_predicate: $ => seq(
      field('subject', $.generic_declaration),
      optional(seq(':', field('bound', $._type))),
    ),

    // Declarations
    function_declaration: $ => seq(
      optional('pub'),
      $.identifier,
      choice(
        seq(
          ':',
          $._type,
          optional($.where_clause),
          optional(seq('=', $.expression)),
        ),
        seq('=', $.expression),
      ),
    ),

    struct_declaration: $ => seq(
      optional('pub'),
      'struct',
      $.type_identifier,
      optional($.generic_declarations),
      repeat(seq(
        optional('<'),
        $.identifier, ':', $._type,
        optional(seq('=', $.expression))
      ))
    ),

    enum_declaration: $ => seq(
      optional($.language_item_marker),
      optional('<'),
      optional('pub'),
      'enum',
      $.type_identifier,
      repeat($.enum_variant)
    ),

    enum_variant: $ => seq(
      optional($.language_item_marker),
      choice(
        $.type_identifier,
        seq($.type_identifier, optional(sepBy(',', $._type))),
        seq($.type_identifier, repeat(seq($.identifier, ':', $._type)))
      )
    ),

    trait_declaration: $ => seq(
      optional($.language_item_marker),
      optional('<'),
      optional('pub'),
      'trait',
      $.type_identifier,
      optional($.generic_declarations),
      optional(seq('for', $.generic_declaration)),
      optional($.where_clause),
      repeat($.trait_member)
    ),

    trait_member: $ => seq(
      optional($.language_item_marker),
      choice(
        seq('type', $.type_identifier),
        seq(
          optional('@'),
          $.identifier, ':', $._type,
          optional($.where_clause),
          optional(seq('=', $.expression))
        )
      )
    ),

    language_item_marker: $ => seq(
      'lang',
      field('role', $.identifier),
      '\n'
    ),

    impl_declaration: $ => seq(
      optional('pub'),
      'impl',
      optional($.type_identifier),
      optional(seq('for', $._type)),
      optional($.where_clause),
      repeat(seq(
        optional('@'),
        $.identifier, ':', $._type,
        optional($.where_clause),
        optional(seq('=', $.expression))
      ))
    ),

    macro_declaration: $ => seq(
      optional('pub'),
      'macro',
      $.identifier,
      repeat(seq('$', $.identifier, ':', $.identifier)),
      '=>',
      repeat1($.expression)
    ),

    infix_declaration: $ => seq(
      'infix',
      $.number,
      $.identifier
    ),

    type_declaration: $ => seq(
      'type',
      $.type_identifier,
      optional($.generic_declarations),
      '=',
      $._type
    ),

    module_declaration: $ => seq('mod', $.identifier),
    extern_declaration: $ => seq('extern', repeat($.function_declaration)),

    // Expressions
    expression: $ => choice(
      $.lambda_expression,
      $.if_expression,
      $.match_expression,
      $.for_expression,
      $.while_expression,
      $.loop_expression,
      $.unsafe_expression,
      $.return_expression,
      $.break_expression,
      $.continue_expression,
      $.binary_expression,
      $.unary_expression,
      $.call_expression,
      $.field_expression,
      $.index_expression,
      $.primary_expression,
    ),

    primary_expression: $ => choice(
      $.identifier,
      $.type_identifier,
      $.number,
      $.float,
      $.string,
      $.char,
      $.boolean,
      $.unit,
      $.array_expression,
      $.parenthesized_expression,
      $.struct_expression,
      $.type_qualified_expression,
      $.macro_invocation,
    ),

    parenthesized_expression: $ => seq('(', $.expression, ')'),
    array_expression: $ => choice(
      seq('[', $.expression, ';', $.number, ']'),
      seq('[', optional(sepBy(',', $.expression)), ']'),
    ),

    struct_expression: $ => seq(
      $.type_identifier,
      '::',
      optional($.identifier),
      optional(sepBy(',', $.expression))
    ),

    type_qualified_expression: $ => prec.dynamic(1, prec(7, seq(
      '(',
      $.type_application,
      ')',
      repeat1(seq('::', choice($.type_identifier, $.identifier))),
    ))),

    macro_invocation: $ => seq('%', $.identifier, repeat($.expression)),

    lambda_expression: $ => seq(
      optional(seq('(', optional(sepBy(',', $.identifier)), ')')),
      choice('->', '!->'),
      $.expression
    ),

    call_expression: $ => choice(
      prec(5, seq(
        $.primary_expression,
        repeat($.expression)
      )),
      prec(6, seq(
        $.primary_expression,
        choice(
          seq($.call_hole, repeat(seq(',', $._call_argument))),
          seq(
            repeat1(seq($.expression, ',')),
            $.call_hole,
            repeat(seq(',', $._call_argument))
          )
        )
      ))
    ),

    _call_argument: $ => choice($.expression, $.call_hole),

    call_hole: $ => '_',

    field_expression: $ => prec(6, seq(
      choice($.primary_expression, $.call_expression),
      optional('!'),
      '.',
      $.identifier
    )),

    index_expression: $ => seq(
      $.expression,
      '[',
      $.expression,
      ']'
    ),

    binary_expression: $ => prec.left(1, seq(
      $.expression,
      $.operator,
      $.expression
    )),

    unary_expression: $ => seq($.operator, $.expression),
    operator: $ => /[+\-*/=!<>|&^:]+|->|=>|::|\.\./,

    if_expression: $ => seq(
      'if', $.expression, 'then', $.expression,
      optional(seq('else', $.expression))
    ),

    match_expression: $ => seq(
      'match',
      $.expression,
      repeat(seq($.pattern, '=>', $.expression))
    ),

    pattern: $ => choice(
      $.identifier,
      $.type_identifier,
      $.number,
      $.string,
      $.boolean,
      seq('(', optional(sepBy(',', $.pattern)), ')'),
      seq('[', optional(sepBy(',', $.pattern)), ']'),
      '_'
    ),

    for_expression: $ => seq('for', $.identifier, 'in', $.expression, $.expression),
    while_expression: $ => seq('while', $.expression, $.expression),
    loop_expression: $ => seq('loop', $.expression),
    unsafe_expression: $ => seq('unsafe', $.expression),
    return_expression: $ => seq('return', optional($.expression)),
    break_expression: $ => 'break',
    continue_expression: $ => 'continue',

    assignment: $ => seq($.identifier, '=', $.expression),
  }
});

function sepBy(separator, rule) {
  return optional(seq(rule, repeat(seq(separator, rule))));
}
