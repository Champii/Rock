(function () {
    "use strict";

    function registerRock() {
        if (typeof hljs === "undefined") {
            return;
        }

        if (!hljs.getLanguage("rock")) {
            hljs.registerLanguage("rock", function (hljs) {
                var TYPE = {
                    className: "type",
                    begin: /\b[A-Z][A-Za-z0-9_]*\b/,
                };
                var DECLARATION = {
                    className: "title function_",
                    begin: /^[ \t]*[a-z_][A-Za-z0-9_]*(?=[ \t]*(?::[^=\n]+)?=)/m,
                };
                var METHOD_MARKER = {
                    className: "symbol",
                    begin: /(?:\^@|~@|@)[a-z_][A-Za-z0-9_]*/,
                };
                var MACRO = {
                    className: "meta",
                    begin: /[%$][A-Za-z_][A-Za-z0-9_]*/,
                };
                var NATIVE = {
                    className: "built_in",
                    begin: /~[A-Z][A-Za-z0-9_]*/,
                };

                return {
                    name: "Rock",
                    aliases: ["rk"],
                    keywords: {
                        keyword:
                            "struct enum trait impl if then else for in while loop macro " +
                            "return continue break infix mod extern match unsafe type mut where as",
                        literal: "true false",
                        type:
                            "I8 I16 I32 I64 U8 U16 U32 U64 F32 F64 Bool Char Str Unit Self",
                    },
                    contains: [
                        hljs.C_LINE_COMMENT_MODE,
                        hljs.C_BLOCK_COMMENT_MODE,
                        hljs.QUOTE_STRING_MODE,
                        {
                            className: "string",
                            begin: /'(?:\\.|[^'\\])'/,
                        },
                        hljs.C_NUMBER_MODE,
                        NATIVE,
                        MACRO,
                        METHOD_MARKER,
                        DECLARATION,
                        TYPE,
                    ],
                };
            });
        }

        document.querySelectorAll("pre code.language-rock, pre code.language-rk").forEach(function (block) {
            if (!block.dataset.highlighted) {
                if (typeof hljs.highlightElement === "function") {
                    hljs.highlightElement(block);
                } else {
                    hljs.highlightBlock(block);
                }
            }
        });
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", registerRock);
    } else {
        registerRock();
    }
})();
