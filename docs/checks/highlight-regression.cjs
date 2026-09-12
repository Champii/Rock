const assert = require("node:assert/strict");
const { htmlSource } = require("./rock-highlight.cjs");

const resourceSource = `struct Resource
    < id: I64

impl Drop for Resource
    ~@drop = -> return

main = !->
    resource = Resource
        id: 7
    resource.id.println!
`;

function verifyResourceHighlight(html) {
    assert.equal(htmlSource(html), resourceSource, "Resource snippet source must survive highlighting exactly");
    const spans = [...html.matchAll(/<span class=['"]([^'"]+)['"]>([^<]*)<\/span>/g)]
        .map((match) => ({ classes: match[1].split(" "), text: htmlSource(match[2]) }));
    for (const [text, capture] of [
        ["struct", "keyword"], ["Resource", "type"], ["<", "keyword.export"],
        ["id", "property"], [":", "punctuation.annotation"], ["I64", "type"],
        ["Drop", "type"], ["~@", "variable.builtin"], ["drop", "function"],
        ["=", "operator.assignment"], ["->", "operator.arrow"], ["println", "function"],
        ["!->", "operator.arrow"],
        ["!", "punctuation.special"], ["resource", "variable"], ["7", "number"],
    ]) {
        assert.ok(spans.some((span) => span.text === text && span.classes.join(".") === capture),
            `Expected AST capture ${capture} for ${JSON.stringify(text)} in Resource snippet`);
    }
}

module.exports = { resourceSource, verifyResourceHighlight };
