#!/usr/bin/env node
/*
 * GATE-04: Strict ES5 & WebKitGTK 1.0 (JavaScriptCore 2012) Compatibility Gate
 * Usage:
 *   node tools/check_es5.js <file.js>            (full check: ES5 + banned APIs + no Emoji)
 *   node tools/check_es5.js --syntax-only <file> (ES5 syntax check only, for vendored libs)
 */
var fs = require('fs');
var acorn = require('acorn');

var args = process.argv.slice(2);
var syntaxOnly = false;
if (args[0] === '--syntax-only') {
    syntaxOnly = true;
    args.shift();
}

if (args.length < 1) {
    console.error("Usage: node check_es5.js [--syntax-only] <file.js>");
    process.exit(1);
}

var filePath = args[0];
console.log("=== GATE-04: Checking ES5 & WebKit 1.0 Compatibility for " + filePath + " ===");

var BANNED_GLOBALS = {
    localStorage: "Throws DOM Exception 18 on file:// in WebKitGTK 1.0",
    sessionStorage: "Unavailable/unsafe on file:// in WebKitGTK 1.0",
    Promise: "Missing in JavaScriptCore 2012",
    fetch: "Missing in WebKitGTK 1.0",
    requestAnimationFrame: "Missing in WebKitGTK 1.0",
    webkitRequestAnimationFrame: "Missing in WebKitGTK 1.0",
    IntersectionObserver: "Missing in WebKitGTK 1.0",
    ResizeObserver: "Missing in WebKitGTK 1.0",
    Symbol: "Missing in JavaScriptCore 2012",
    Map: "Missing in JavaScriptCore 2012",
    Set: "Missing in JavaScriptCore 2012"
};

var BANNED_PROPERTIES = {
    closest: "Element.prototype.closest() missing in WebKitGTK 1.0 (2012)",
    startsWith: "String.prototype.startsWith() missing in JavaScriptCore 2012",
    endsWith: "String.prototype.endsWith() missing in JavaScriptCore 2012",
    includes: "Array/String.prototype.includes() missing in JavaScriptCore 2012",
    findIndex: "Array.prototype.findIndex() missing in JavaScriptCore 2012",
    assign: "Object.assign() missing in JavaScriptCore 2012"
};

// Kindle OS 5.14 lacks Emoji fonts (renders tofu □)
var EMOJI_RE = /[\u2600-\u27BF]|[\uD83C-\uDBFF][\uDC00-\uDFFF]/;

function walk(node, visitor, parent) {
    if (!node || typeof node !== 'object') return;
    visitor(node, parent);
    for (var key in node) {
        if (!node.hasOwnProperty(key) || key === 'loc' || key === 'start' || key === 'end') continue;
        var child = node[key];
        if (Array.isArray(child)) {
            for (var i = 0; i < child.length; i++) {
                walk(child[i], visitor, node);
            }
        } else if (child && typeof child.type === 'string') {
            walk(child, visitor, node);
        }
    }
}

try {
    var code = fs.readFileSync(filePath, 'utf8');
    var ast = acorn.parse(code, { ecmaVersion: 5, locations: true });
    console.log("[PASS] 100% valid ES5 syntax!");

    if (syntaxOnly) {
        process.exit(0);
    }

    var violations = [];
    walk(ast, function (node, parent) {
        var line = node.loc ? node.loc.start.line : '?';

        if (node.type === 'Identifier' && BANNED_GLOBALS.hasOwnProperty(node.name)) {
            // Ignore property names or declaration names that aren't global reads
            var isPropertyKey = parent && parent.type === 'Property' && parent.key === node;
            var isMemberProp = parent && parent.type === 'MemberExpression' && parent.property === node && !parent.computed;
            if (!isPropertyKey && !isMemberProp) {
                violations.push("Line " + line + ": Banned global '" + node.name + "' — " + BANNED_GLOBALS[node.name]);
            }
        }

        if (node.type === 'MemberExpression' && !node.computed && node.property && node.property.type === 'Identifier') {
            var prop = node.property.name;
            if (BANNED_PROPERTIES.hasOwnProperty(prop)) {
                violations.push("Line " + line + ": Banned method/property '." + prop + "' — " + BANNED_PROPERTIES[prop]);
            }
            if (node.object && node.object.type === 'Identifier' && node.object.name === 'performance' && prop === 'now') {
                violations.push("Line " + line + ": Banned 'performance.now' — missing in WebKitGTK 1.0 (use Date.now() / new Date().getTime())");
            }
        }

        if (node.type === 'Literal' && typeof node.value === 'string') {
            if (EMOJI_RE.test(node.value)) {
                violations.push("Line " + line + ": Unicode Emoji literal detected (renders tofu box on Kindle OS): " + JSON.stringify(node.value));
            }
        }
    }, null);

    if (violations.length > 0) {
        console.error("[FAIL] WebKit 1.0 / Kindle OS Compatibility Violations in " + filePath + ":");
        for (var v = 0; v < violations.length; v++) {
            console.error("  - " + violations[v]);
        }
        process.exit(1);
    }

    console.log("[PASS] Zero banned post-2012 APIs and zero Unicode Emoji literals!");
    process.exit(0);
} catch (err) {
    console.error("[FAIL] ES6+ Syntax Violation in " + filePath + ":");
    console.error("  " + err.message);
    if (err.loc) {
        console.error("  Line: " + err.loc.line + ", Column: " + err.loc.column);
    }
    process.exit(1);
}
