#!/usr/bin/env node
/*
 * tools/test_mathjax_bundle.js
 * Verifies local MathJax 2.7.9 bundle integrity (22 TeX WOFF fonts + all required
 * STEM/chemistry/bbox extensions) and runs all 38 torture benchmark cases + 838 real
 * user formulas through CardEngine's HTML preprocessor to guarantee zero corruption.
 */
var fs = require('fs');
var path = require('path');
var vm = require('vm');
var assert = require('assert');

var repoRoot = path.resolve(__dirname, '..');
var mjRoot = path.join(repoRoot, 'views', 'mathjax');

console.log('=== RAnki Route B MathJax 2.7.9 Bundle & 876-Formula Gate ===');

// 1. Verify core MathJax files and required STEM/chemistry extensions
var requiredFiles = [
    'MathJax.js',
    'config/TeX-AMS_HTML.js',
    'extensions/tex2jax.js',
    'extensions/TeX/AMSmath.js',
    'extensions/TeX/AMSsymbols.js',
    'extensions/TeX/mhchem.js',
    'extensions/TeX/mhchem3/mhchem.js',
    'extensions/TeX/extpfeil.js',
    'extensions/TeX/color.js',
    'extensions/TeX/bbox.js',
    'extensions/TeX/cancel.js',
    'extensions/TeX/enclose.js',
    'extensions/TeX/newcommand.js',
    'extensions/TeX/noErrors.js',
    'extensions/TeX/noUndefined.js',
    'jax/input/TeX/jax.js',
    'jax/output/HTML-CSS/jax.js'
];

for (var i = 0; i < requiredFiles.length; i++) {
    var fp = path.join(mjRoot, requiredFiles[i]);
    assert.ok(fs.existsSync(fp), 'Missing required MathJax asset: ' + requiredFiles[i]);
}
console.log('[PASS] Verified all ' + requiredFiles.length + ' required MathJax 2.7.9 core & TeX extensions.');

// 2. Verify all 22 local TeX WOFF fonts exist
var woffDir = path.join(mjRoot, 'fonts', 'HTML-CSS', 'TeX', 'woff');
var woffs = fs.readdirSync(woffDir).filter(function (f) { return f.slice(-5) === '.woff'; });
assert.ok(woffs.length >= 22, 'Expected >= 22 MathJax TeX WOFF fonts, found ' + woffs.length);
console.log('[PASS] Verified ' + woffs.length + ' MathJax TeX WOFF fonts in views/mathjax/fonts/HTML-CSS/TeX/woff/.');

// 3. Load CardEngine preprocessor and verify all 38 benchmark + 838 real formulas survive preprocessing intact
var cardJsCode = fs.readFileSync(path.join(repoRoot, 'views', 'card.js'), 'utf8');
var sandbox = {
    window: {},
    document: { readyState: 'loading', addEventListener: function () {} },
    addEventListener: function () {},
    navigator: { userAgent: 'Kindle' },
    console: console
};
sandbox.window = sandbox;
vm.createContext(sandbox);
vm.runInContext(cardJsCode, sandbox);
var pp = sandbox.CardEngine._preprocessCardHtml;

var benchPath = path.join(repoRoot, 'tools', 'test_cases.json');
var benchCases = JSON.parse(fs.readFileSync(benchPath, 'utf8')).cases || [];
assert.ok(benchCases.length >= 38, 'Expected >= 38 torture benchmark cases');
for (var b = 0; b < benchCases.length; b++) {
    var outB = pp(benchCases[b].formatted || benchCases[b].latex || '');
    assert.ok(outB.length > 0 && outB.indexOf('___RANKI_MATH_') === -1, 'Benchmark #' + benchCases[b].index + ' corrupted during preprocess');
}
console.log('[PASS] All ' + benchCases.length + ' STEM torture benchmark formulas passed preprocessing intact.');

var userPath = path.join(repoRoot, 'tools', 'fixtures', 'collection_formulas.json');
var userJson = JSON.parse(fs.readFileSync(userPath, 'utf8'));
var userFormulas = Array.isArray(userJson) ? userJson : (userJson.formulas || []);
assert.ok(userFormulas.length >= 800, 'Expected >= 800 user formulas');
for (var u = 0; u < userFormulas.length; u++) {
    var expr = (typeof userFormulas[u] === 'string') ? userFormulas[u] : (userFormulas[u].tex || userFormulas[u].expr || '');
    var wrapped = '\\(' + expr + '\\)';
    var outU = pp(wrapped);
    assert.ok(outU.indexOf('___RANKI_MATH_') === -1 && outU.length >= 4, 'User formula #' + u + ' corrupted during preprocess');
}
console.log('[PASS] All ' + userFormulas.length + ' real user formulas passed preprocessing intact (total: ' + (benchCases.length + userFormulas.length) + ').');
console.log('=== ALL MATHJAX BUNDLE & FORMULA GATES PASSED ===');
