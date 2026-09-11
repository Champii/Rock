const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const test = require("node:test");
const { rockFences, htmlSource, highlightSources, preprocess } = require("./rock-highlight.cjs");
const { resourceSource, verifyResourceHighlight } = require("./highlight-regression.cjs");

const script = path.join(__dirname, "rock-highlight.cjs");
function book(content) {
    return { sections: [{ Chapter: { name: "Test", content, sub_items: [] } }] };
}

test("mdbook protocol supports only HTML and works outside the repository", () => {
    for (const renderer of ["html", "markdown"]) {
        const result = spawnSync(process.execPath, [script, "supports", renderer], { cwd: os.tmpdir() });
        assert.equal(result.status, renderer === "html" ? 0 : 1);
        assert.equal(result.stdout.toString(), "");
    }
    const result = spawnSync(process.execPath, [script], {
        cwd: os.tmpdir(), encoding: "utf8",
        input: JSON.stringify([{ renderer: "html" }, book("```rock\n" + resourceSource + "```\n")]),
    });
    assert.equal(result.status, 0, result.stderr);
    const content = JSON.parse(result.stdout).sections[0].Chapter.content;
    verifyResourceHighlight(content.match(/<code[^>]*>([\s\S]*?)<\/code>/)[1]);
});

test("fences respect delimiter lengths, non-Rock blocks, and unclosed fences", () => {
    const markdown = "````markdown\n```rock\nnot Rock\n```\n````\n" +
        "`````rock\n// ```\nmain = ->\n    0\n`````\n" +
        "~~~rock\n// tilde fence\n~~~\n```rust\nlet x = 1;\n```\n";
    assert.deepEqual(rockFences(markdown).map((block) => block.source), ["// ```\nmain = ->\n    0\n", "// tilde fence\n"]);
    assert.equal(rockFences("```rock\nmain = -> 0")[0].source, "main = -> 0");
    assert.equal(rockFences("```rk\nmain = -> 0\n```\n")[0].source, "main = -> 0\n");
    assert.deepEqual(rockFences("    ```rock\n    not a fence\n    ```\n"), []);
    const unchanged = book("```rust\nlet x = 1;\n```\n");
    assert.deepEqual(preprocess({ renderer: "html" }, structuredClone(unchanged)), unchanged);
    assert.deepEqual(preprocess({ renderer: "markdown" }, structuredClone(unchanged)), unchanged);
    const modernBook = { items: unchanged.sections };
    assert.deepEqual(preprocess({ renderer: "html" }, structuredClone(modernBook)), modernBook);
});

test("batch highlighting preserves blank lines, multiline strings, entities, tabs, Unicode, and EOF", () => {
    const sources = [resourceSource, "", "\n\n", "// & < > \" ' café\n\nmain = ->\n\t0  \n",
        'main = ->\n    "first\nsecond <&>"\n    0', "// Windows\r\nmain = ->\r\n    0\r\n"];
    const highlighted = highlightSources(sources);
    assert.deepEqual(highlighted.map(htmlSource), sources);
    verifyResourceHighlight(highlighted[0]);
    assert.ok(!highlighted.some((html) => /<table|class=line|<style/.test(html)));
});

test("nested chapters use nohighlight and leave other Markdown untouched", () => {
    const input = book("Before\n\n```rust\nlet x = 1;\n```\n\nAfter\n");
    input.sections[0].Chapter.sub_items = book("````rock\n" + resourceSource + "````\n").sections;
    const result = preprocess({ renderer: "html" }, structuredClone(input));
    assert.equal(result.sections[0].Chapter.content, input.sections[0].Chapter.content);
    assert.match(result.sections[0].Chapter.sub_items[0].Chapter.content,
        /<pre><code class="language-rock nohighlight" data-rock-highlight="tree-sitter">/);
});

test("CLI failures are helpful, leave stdout empty, and clean temporary files", () => {
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "rock-book-test-"));
    try {
        const result = spawnSync(process.execPath, [script], {
            env: { ...process.env, PATH: temporary, TMPDIR: temporary }, encoding: "utf8",
            input: JSON.stringify([{ renderer: "html" }, book("```rock\nmain = -> 0\n```\n")]),
        });
        assert.equal(result.status, 1);
        assert.equal(result.stdout, "");
        assert.match(result.stderr, /Install tree-sitter-cli 0\.26\.9/);
        assert.deepEqual(fs.readdirSync(temporary), []);
    } finally {
        fs.rmSync(temporary, { recursive: true, force: true });
    }
});

test("colorful palettes exceed AA text contrast in dark and light themes", () => {
    const css = fs.readFileSync(path.join(__dirname, "../theme/rock.css"), "utf8");
    function luminance(hex) {
        const rgb = hex.match(/[\da-f]{2}/g).map((channel) => {
            const value = parseInt(channel, 16) / 255;
            return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
        });
        return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
    }
    const palettes = [...css.matchAll(/\{([^{}]*--rock-code-bg:[^{}]*)\}/g)];
    assert.equal(palettes.length, 2);
    for (const [, palette] of palettes) {
        const colors = Object.fromEntries([...palette.matchAll(/--rock-code-([\w-]+):\s*#([\da-f]{6})/g)]
            .map((match) => [match[1], luminance(match[2])]));
        for (const [name, value] of Object.entries(colors)) {
            if (name === "bg" || name === "border") continue;
            const contrast = (Math.max(value, colors.bg) + 0.05) / (Math.min(value, colors.bg) + 0.05);
            const minimum = name === "comment" ? 4.5 : 5.5;
            assert.ok(contrast >= minimum, `${name} contrast ${contrast.toFixed(2)} must be at least ${minimum}:1`);
        }
    }
});
