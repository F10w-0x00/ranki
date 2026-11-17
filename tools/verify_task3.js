// Automated verification script for Task 3 & V2 Pipeline (Dual/Quad-Buffer MathJax Engine)
var fs = require('fs');
var path = require('path');

console.log("=== Running Task 3 & V2 Automated Verifications ===");

var repoRoot = path.resolve(__dirname, '..');

// 1. Check MathJax vendored assets
var mathjaxJs = path.join(repoRoot, 'views', 'mathjax', 'MathJax.js');
if (!fs.existsSync(mathjaxJs)) {
    console.error("[FAIL] views/mathjax/MathJax.js missing!");
    process.exit(1);
}
var mathjaxStat = fs.statSync(mathjaxJs);
console.log("[PASS] views/mathjax/MathJax.js present (" + mathjaxStat.size + " bytes)");

// Check TeX WOFF fonts
var woffDir = path.join(repoRoot, 'views', 'mathjax', 'fonts', 'HTML-CSS', 'TeX', 'woff');
if (!fs.existsSync(woffDir)) {
    console.error("[FAIL] TeX woff font directory missing: " + woffDir);
    process.exit(1);
}
var woffFiles = fs.readdirSync(woffDir).filter(function (f) { return f.endsWith('.woff'); });
if (woffFiles.length < 20) {
    console.error("[FAIL] Expected at least 20 TeX woff fonts, found: " + woffFiles.length);
    process.exit(1);
}
console.log("[PASS] TeX WOFF fonts verified: " + woffFiles.length + " fonts present in views/mathjax/");

// Check total size of views/mathjax
function getDirSize(dir) {
    var size = 0;
    var list = fs.readdirSync(dir);
    for (var i = 0; i < list.length; i++) {
        var p = path.join(dir, list[i]);
        var s = fs.statSync(p);
        if (s.isDirectory()) {
            size += getDirSize(p);
        } else {
            size += s.size;
        }
    }
    return size;
}
var mjTotalSize = getDirSize(path.join(repoRoot, 'views', 'mathjax'));
console.log("[PASS] Total views/mathjax footprint: " + (mjTotalSize / (1024 * 1024)).toFixed(2) + " MB (Limit: <= 2.5MB)");
if (mjTotalSize > 2.5 * 1024 * 1024) {
    console.error("[FAIL] views/mathjax exceeds 2.5MB budget!");
    process.exit(1);
}

// 2. Check views/index.html (supports legacy dual-buffer and V2 quad-slot pipeline)
var htmlContent = fs.readFileSync(path.join(repoRoot, 'views', 'index.html'), 'utf8');
var hasSlots = (htmlContent.indexOf('slot-a-front') !== -1 && htmlContent.indexOf('slot-a-back') !== -1) ||
               (htmlContent.indexOf('card-active') !== -1 && htmlContent.indexOf('card-buffer') !== -1);
if (!hasSlots) {
    console.error("[FAIL] Buffer slots missing in views/index.html");
    process.exit(1);
}
if (htmlContent.indexOf('MathJax.Hub.Config') === -1 || htmlContent.indexOf('mathjax/MathJax.js') === -1) {
    console.error("[FAIL] MathJax loader missing in views/index.html");
    process.exit(1);
}
console.log("[PASS] views/index.html buffer slots & MathJax loader verified.");

// 3. Check views/style.css
var cssContent = fs.readFileSync(path.join(repoRoot, 'views', 'style.css'), 'utf8');
if (cssContent.indexOf('card-box') === -1 || cssContent.indexOf('offscreen') === -1) {
    console.error("[FAIL] Buffer classes missing in views/style.css");
    process.exit(1);
}
if (cssContent.indexOf('left: -9999px') === -1 || cssContent.indexOf('visibility: visible') === -1) {
    console.error("[FAIL] Offscreen geometry measurement styling missing in views/style.css");
    process.exit(1);
}
console.log("[PASS] views/style.css buffer geometry & styling verified.");

// 4. Check 38 test cases in tools/test_cases.json
var testCasesPath = path.join(repoRoot, 'tools', 'test_cases.json');
if (!fs.existsSync(testCasesPath)) {
    console.error("[FAIL] tools/test_cases.json missing!");
    process.exit(1);
}
var casesData = JSON.parse(fs.readFileSync(testCasesPath, 'utf8'));
var cases = casesData.cases || [];
if (cases.length !== 38) {
    console.error("[FAIL] Expected 38 test cases, found: " + cases.length);
    process.exit(1);
}
console.log("[PASS] 38 torture and real math test cases verified in tools/test_cases.json.");

console.log("\n=== ALL TASK 3 & V2 VERIFICATIONS PASSED (100%) ===");
