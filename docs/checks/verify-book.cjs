const fs = require("fs");
const path = require("path");
const vm = require("vm");

const docsRoot = path.resolve(__dirname, "..");
const sourceRoot = path.join(docsRoot, "src");
const outputRoot = path.join(docsRoot, "book");

function walk(directory, predicate) {
    const results = [];
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
        const entryPath = path.join(directory, entry.name);
        if (entry.isDirectory()) {
            results.push(...walk(entryPath, predicate));
        } else if (predicate(entryPath)) {
            results.push(entryPath);
        }
    }
    return results;
}

function fail(message) {
    console.error(message);
    process.exit(1);
}

if (!fs.existsSync(outputRoot)) {
    fail("The rendered book is missing; run `mdbook build docs` first.");
}

const summaryPath = path.join(sourceRoot, "SUMMARY.md");
const summary = fs.readFileSync(summaryPath, "utf8");
const chapterLinks = [...summary.matchAll(/\]\(([^)]+\.md)\)/g)].map((match) => match[1]);
const missingChapters = chapterLinks.filter((chapter) => !fs.existsSync(path.join(sourceRoot, chapter)));
if (missingChapters.length > 0) {
    fail(`Missing SUMMARY targets:\n${missingChapters.join("\n")}`);
}

const markdownFiles = walk(sourceRoot, (file) => file.endsWith(".md"));
const listedChapters = new Set(chapterLinks.map((chapter) => path.resolve(sourceRoot, chapter)));
const orphanedChapters = markdownFiles.filter(
    (file) => file !== summaryPath && !listedChapters.has(path.resolve(file)),
);
if (orphanedChapters.length > 0) {
    fail(`Markdown pages missing from SUMMARY.md:\n${orphanedChapters.join("\n")}`);
}

let rockFenceCount = 0;
for (const file of markdownFiles) {
    const source = fs.readFileSync(file, "utf8");
    if (/\blang\b/.test(source)) {
        fail(`Compiler-internal \`lang\` syntax is not allowed in the user book: ${file}`);
    }
    if (/--debug-print|\bHIR\b|\bMIR\b|ast-full/.test(source)) {
        fail(`Compiler-internal debug views are not allowed in the user book: ${file}`);
    }

    const rockFences = [...source.matchAll(/```rock\b[^\n]*\n([\s\S]*?)```/g)];
    for (const fence of rockFences) {
        if (/\.\.\.|\bTODO\b|\/\/\s*(?:body|implementation|omitted)\b/i.test(fence[1])) {
            fail(`Rock fence contains an omission or placeholder in ${file}:\n${fence[1]}`);
        }
    }
    rockFenceCount += rockFences.length;
}
if (rockFenceCount === 0) {
    fail("The book contains no Rock code fences.");
}

const bookConfig = fs.readFileSync(path.join(docsRoot, "book.toml"), "utf8");
const siteUrlMatch = bookConfig.match(/^site-url\s*=\s*"([^"]+)"/m);
const siteUrl = siteUrlMatch ? siteUrlMatch[1] : "/";
const htmlFiles = walk(outputRoot, (file) => file.endsWith(".html"));
const brokenLinks = [];
let renderedRockBlocks = 0;

for (const file of htmlFiles) {
    const html = fs.readFileSync(file, "utf8");
    renderedRockBlocks += (html.match(/class="language-rock"/g) || []).length;

    for (const match of html.matchAll(/href="([^"]+)"/g)) {
        const href = match[1];
        if (/^(?:https?:|mailto:|javascript:|#)/.test(href)) {
            continue;
        }

        const target = href.split(/[?#]/)[0];
        if (!target) {
            continue;
        }

        let resolved;
        if (siteUrl !== "/" && target.startsWith(siteUrl)) {
            resolved = path.join(outputRoot, target.slice(siteUrl.length) || "index.html");
        } else {
            resolved = path.resolve(path.dirname(file), target);
        }

        if (!fs.existsSync(resolved)) {
            brokenLinks.push(`${path.relative(outputRoot, file)} -> ${href}`);
        }
    }
}

if (brokenLinks.length > 0) {
    fail(`Broken rendered links:\n${brokenLinks.join("\n")}`);
}
if (renderedRockBlocks < rockFenceCount) {
    fail(`Only ${renderedRockBlocks} of ${rockFenceCount} Rock fences were rendered with language-rock.`);
}

const indexHtml = fs.readFileSync(path.join(outputRoot, "index.html"), "utf8");
// mdBook can fingerprint asset names; validate and load the rendered paths.
const assets = [
    /href="(theme\/rock(?:-[a-f0-9]+)?\.css)"/,
    /src="(theme\/rock-highlight(?:-[a-f0-9]+)?\.js)"/,
    /src="(highlight(?:-[a-f0-9]+)?\.js)"/,
].map((pattern) => {
    const match = indexHtml.match(pattern);
    if (!match || !fs.existsSync(path.join(outputRoot, match[1]))) {
        fail(`The rendered book is missing a required asset: ${pattern}`);
    }
    return path.join(outputRoot, match[1]);
});

const context = {
    console,
    document: {
        readyState: "complete",
        querySelectorAll() {
            return [];
        },
        addEventListener() {},
    },
};
context.window = context;
context.self = context;
vm.createContext(context);
vm.runInContext(fs.readFileSync(assets[2], "utf8"), context);
vm.runInContext(fs.readFileSync(assets[1], "utf8"), context);

if (!context.hljs || !context.hljs.getLanguage("rock")) {
    fail("The custom Rock Highlight.js grammar was not registered.");
}
const highlighted = context.hljs.highlight("rock", 'main = ->\n    "Hello".println!\n    0', true);
if (!highlighted.value.includes("hljs-")) {
    fail("The Rock grammar did not highlight any tokens.");
}

console.log(
    `Verified ${chapterLinks.length} chapters, ${rockFenceCount} Rock fences, ` +
        `${htmlFiles.length} HTML pages, rendered links, and Rock syntax highlighting.`,
);
