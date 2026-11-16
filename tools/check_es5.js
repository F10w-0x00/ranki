#!/usr/bin/env node
var fs = require('fs');
var acorn = require('acorn');

if (process.argv.length < 3) {
    console.error("Usage: node check_es5.js <file.js>");
    process.exit(1);
}

var filePath = process.argv[2];
console.log("=== Checking ES5 Syntax for " + filePath + " ===");

try {
    var code = fs.readFileSync(filePath, 'utf8');
    acorn.parse(code, { ecmaVersion: 5 });
    console.log("[PASS] 100% valid ES5 syntax! No ES6+ features detected.");
    process.exit(0);
} catch (err) {
    console.error("[FAIL] ES6+ Syntax Violation in " + filePath + ":");
    console.error("  " + err.message);
    if (err.loc) {
        console.error("  Line: " + err.loc.line + ", Column: " + err.loc.column);
    }
    process.exit(1);
}
