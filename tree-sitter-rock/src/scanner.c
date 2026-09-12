#include "tree_sitter/array.h"
#include "tree_sitter/parser.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

enum TokenType {
    NEWLINE,
    INDENT,
    CALL_INDENT,
    DEDENT,
    SPACED_DOT,
    SPACED_OPERATOR,
    STUCK_OPERATOR,
    POSTFIX_CONTINUATION,
    MACRO_SIGIL,
};

typedef struct {
    Array(uint16_t) indents;
    uint16_t pending_dedents;
    bool pending_newline;
} Scanner;

static void advance(TSLexer *lexer) {
    lexer->advance(lexer, false);
}

static void skip(TSLexer *lexer) {
    lexer->advance(lexer, true);
}

bool tree_sitter_rock_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    Scanner *scanner = payload;

    if (scanner->pending_dedents > 0 && valid_symbols[DEDENT]) {
        scanner->pending_dedents--;
        lexer->mark_end(lexer);
        lexer->result_symbol = DEDENT;
        return true;
    }
    if (scanner->pending_dedents == 0 && scanner->pending_newline &&
        valid_symbols[NEWLINE]) {
        scanner->pending_newline = false;
        lexer->mark_end(lexer);
        lexer->result_symbol = NEWLINE;
        return true;
    }
    if (scanner->pending_dedents == 0 && valid_symbols[DEDENT] &&
        scanner->indents.size > 1 &&
        (lexer->lookahead == ')' || lexer->lookahead == ']' ||
         lexer->lookahead == '}')) {
        array_pop(&scanner->indents);
        lexer->mark_end(lexer);
        lexer->result_symbol = DEDENT;
        return true;
    }

    if (valid_symbols[POSTFIX_CONTINUATION] && lexer->lookahead == '.') {
        advance(lexer);
        if (lexer->lookahead != '.') {
            return false;
        }
        advance(lexer);
        if (!((lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
              lexer->lookahead == '_')) {
            return false;
        }
        do {
            advance(lexer);
        } while ((lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
                 (lexer->lookahead >= 'A' && lexer->lookahead <= 'Z') ||
                 (lexer->lookahead >= '0' && lexer->lookahead <= '9') ||
                 lexer->lookahead == '_');
        if (lexer->lookahead == '?' || lexer->lookahead == '!') {
            advance(lexer);
        }
        lexer->mark_end(lexer);
        lexer->result_symbol = POSTFIX_CONTINUATION;
        return true;
    }

    if (valid_symbols[MACRO_SIGIL] && lexer->lookahead == '%') {
        advance(lexer);
        if ((lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
            lexer->lookahead == '_') {
            lexer->mark_end(lexer);
            lexer->result_symbol = MACRO_SIGIL;
            return true;
        }
        if (valid_symbols[SPACED_OPERATOR] &&
            (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
             lexer->lookahead == '\r' || lexer->lookahead == '\n' ||
             lexer->eof(lexer))) {
            lexer->mark_end(lexer);
            lexer->result_symbol = SPACED_OPERATOR;
            return true;
        }
        return false;
    }

    lexer->mark_end(lexer);
    bool found_newline = false;
    bool saw_physical_newline = false;
    bool at_eof = false;
    uint16_t indent = 0;

    for (;;) {
        if (!found_newline &&
            (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
             lexer->lookahead == '\f')) {
            skip(lexer);
        } else if (lexer->lookahead == '\n') {
            found_newline = true;
            saw_physical_newline = true;
            indent = 0;
            skip(lexer);
        } else if (lexer->lookahead == '\r') {
            found_newline = true;
            saw_physical_newline = true;
            indent = 0;
            skip(lexer);
            if (lexer->lookahead == '\n') {
                skip(lexer);
            }
        } else if (found_newline && lexer->lookahead == ' ') {
            indent++;
            skip(lexer);
        } else if (found_newline && lexer->lookahead == '\t') {
            indent = (uint16_t)((indent + 8) & ~7);
            skip(lexer);
        } else if (found_newline && lexer->lookahead == '/' ) {
            skip(lexer);
            if (lexer->lookahead != '/') {
                return false;
            }
            while (lexer->lookahead && lexer->lookahead != '\n' &&
                   lexer->lookahead != '\r') {
                skip(lexer);
            }
        } else if (lexer->eof(lexer)) {
            found_newline = true;
            at_eof = true;
            indent = 0;
            break;
        } else {
            break;
        }
    }

    if (!found_newline) {
        // A prefix operator must touch its operand. Spaced operators belong to
        // binary expressions, not to a call's unary argument (`x + 1`).
        if ((valid_symbols[STUCK_OPERATOR] || valid_symbols[SPACED_OPERATOR]) &&
            lexer->lookahead &&
            strchr("+-*/%=!<>$|&;^~", lexer->lookahead)) {
            char prefix[4] = {0};
            unsigned length = 0;
            do {
                if (length < sizeof(prefix) - 1) {
                    prefix[length] = (char)lexer->lookahead;
                }
                length++;
                advance(lexer);
            } while (lexer->lookahead && strchr("+-*/%=!<>$|&;^~", lexer->lookahead));
            if ((length <= 3 && (strcmp(prefix, "=") == 0 ||
                                 strcmp(prefix, ";") == 0 ||
                                 strcmp(prefix, "->") == 0 ||
                                 strcmp(prefix, "!->") == 0 ||
                                 strcmp(prefix, "~>") == 0 ||
                                 strcmp(prefix, "=>") == 0)) ||
                (length == 1 && prefix[0] == '!' && valid_symbols[SPACED_OPERATOR]) ||
                ((strcmp(prefix, "~") == 0 || strcmp(prefix, "!~") == 0) &&
                 lexer->lookahead >= 'A' && lexer->lookahead <= 'Z')) {
                return false;
            }
            lexer->mark_end(lexer);
            if (lexer->eof(lexer) || strchr(" \t\r\n\f", lexer->lookahead)) {
                if (!valid_symbols[SPACED_OPERATOR] || (length == 1 && prefix[0] == '!')) {
                    return false;
                }
                lexer->result_symbol = SPACED_OPERATOR;
                return true;
            }
            if (!valid_symbols[STUCK_OPERATOR]) {
                return false;
            }
            // Leave the mutable-borrow keyword to the grammar's `&mut` branch.
            if (length == 1 && prefix[0] == '&' && lexer->lookahead == 'm') {
                advance(lexer);
                if (lexer->lookahead == 'u') {
                    advance(lexer);
                    if (lexer->lookahead == 't') {
                        advance(lexer);
                        if (lexer->lookahead && strchr(" \t\r\n", lexer->lookahead)) {
                            return false;
                        }
                    }
                }
            }
            lexer->result_symbol = STUCK_OPERATOR;
            return true;
        }
        return false;
    }

    lexer->mark_end(lexer);

    uint16_t current = *array_back(&scanner->indents);
    if ((valid_symbols[INDENT] || valid_symbols[CALL_INDENT]) && indent > current) {
        array_push(&scanner->indents, indent);
        lexer->result_symbol = valid_symbols[INDENT] ? INDENT : CALL_INDENT;
        return true;
    }
    if (valid_symbols[DEDENT] && indent < current) {
        uint16_t dedent_count = 0;
        for (uint32_t i = scanner->indents.size; i > 1; i--) {
            if (*array_get(&scanner->indents, i - 1) <= indent) {
                break;
            }
            dedent_count++;
        }
        for (uint16_t i = 0; i < dedent_count; i++) {
            array_pop(&scanner->indents);
        }
        scanner->pending_dedents = dedent_count > 0 ? dedent_count - 1 : 0;
        scanner->pending_newline = saw_physical_newline;
        lexer->result_symbol = DEDENT;
        return true;
    }
    if (valid_symbols[NEWLINE] && (!at_eof || saw_physical_newline)) {
        lexer->result_symbol = NEWLINE;
        return true;
    }

    return false;
}

unsigned tree_sitter_rock_external_scanner_serialize(
    void *payload,
    char *buffer
) {
    Scanner *scanner = payload;
    unsigned size = 0;

    buffer[size++] = (char)(scanner->pending_dedents & 0xff);
    buffer[size++] = (char)(scanner->pending_dedents >> 8);
    buffer[size++] = scanner->pending_newline ? 1 : 0;

    for (uint32_t i = 1;
         i < scanner->indents.size &&
         size + 1 < TREE_SITTER_SERIALIZATION_BUFFER_SIZE;
         i++) {
        uint16_t indent = *array_get(&scanner->indents, i);
        buffer[size++] = (char)(indent & 0xff);
        buffer[size++] = (char)(indent >> 8);
    }
    return size;
}

void tree_sitter_rock_external_scanner_deserialize(
    void *payload,
    const char *buffer,
    unsigned length
) {
    Scanner *scanner = payload;
    array_delete(&scanner->indents);
    array_init(&scanner->indents);
    array_push(&scanner->indents, 0);
    scanner->pending_dedents = 0;
    scanner->pending_newline = false;

    unsigned offset = 0;
    if (length >= 3) {
        scanner->pending_dedents = (uint8_t)buffer[0] |
                                   (uint16_t)((uint8_t)buffer[1] << 8);
        scanner->pending_newline = buffer[2] != 0;
        offset = 3;
    }

    for (unsigned i = offset; i + 1 < length; i += 2) {
        uint16_t indent = (uint8_t)buffer[i] |
                          (uint16_t)((uint8_t)buffer[i + 1] << 8);
        array_push(&scanner->indents, indent);
    }
}

void *tree_sitter_rock_external_scanner_create(void) {
    Scanner *scanner = calloc(1, sizeof(Scanner));
    array_init(&scanner->indents);
    array_push(&scanner->indents, 0);
    return scanner;
}

void tree_sitter_rock_external_scanner_destroy(void *payload) {
    Scanner *scanner = payload;
    array_delete(&scanner->indents);
    free(scanner);
}
