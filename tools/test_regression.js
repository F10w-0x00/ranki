#!/usr/bin/env node
/*
 * tools/test_regression.js
 * RAnki Route B — ES5 KaTeX + mhchem + \bbox + WebKit 1.0 vlist-t2 Regression Suite
 */
var fs = require('fs');
var path = require('path');
var vm = require('vm');

var projectRoot = path.resolve(__dirname, '..');
var assetsDir = path.join(projectRoot, 'views', 'katex');

console.log('=== RAnki Route B KaTeX Compatibility & Regression Test ===');

var katexJsPath = path.join(assetsDir, 'katex.min.js');
var mhchemJsPath = path.join(assetsDir, 'mhchem.min.js');
var autoJsPath = path.join(assetsDir, 'auto-render.min.js');

if (!fs.existsSync(katexJsPath) || !fs.existsSync(mhchemJsPath) || !fs.existsSync(autoJsPath)) {
    console.error('[FAIL] KaTeX JS assets missing from ' + assetsDir);
    process.exit(1);
}

// Verify all 20 WOFF fonts present
var fontsDir = path.join(assetsDir, 'fonts');
var woffFiles = fs.readdirSync(fontsDir).filter(function (f) { return f.slice(-5) === '.woff'; });
if (woffFiles.length !== 20) {
    console.error('[FAIL] Expected 20 WOFF fonts in ' + fontsDir + ', found ' + woffFiles.length);
    process.exit(1);
}
console.log('[PASS] Verified 20 KaTeX WOFF fonts in views/katex/fonts/.');

// Build sandbox
var sandbox = {
    window: {},
    document: {
        createElement: function () { return {}; },
        createDocumentFragment: function () { return { appendChild: function () {} }; }
    },
    navigator: {},
    console: console
};
sandbox.window = sandbox;
sandbox.__RANKI_WEBKIT1_VLIST_PATCH__ = false;
vm.createContext(sandbox);

vm.runInContext(fs.readFileSync(katexJsPath, 'utf8'), sandbox);
vm.runInContext(fs.readFileSync(mhchemJsPath, 'utf8'), sandbox);
vm.runInContext(fs.readFileSync(autoJsPath, 'utf8'), sandbox);

var katex = sandbox.katex;
if (!katex || typeof katex.renderToString !== 'function') {
    console.error('[FAIL] KaTeX failed to initialize in sandbox!');
    process.exit(1);
}

var macros = {
    '\\bbox': function (context) {
        if (context.future().text === '[') {
            context.popToken();
            while (context.future().text !== ']' && context.future().text !== 'EOF') {
                context.popToken();
            }
            if (context.future().text === ']') {
                context.popToken();
            }
        }
        return '\\boxed';
    }
};

// 1. Verify conditional WebKit 1.0 vlist-t2 patch toggles cleanly
sandbox.__RANKI_WEBKIT1_VLIST_PATCH__ = false;
var htmlModern = katex.renderToString('x_1 + \\frac{a}{b}', { output: 'html', throwOnError: true });
sandbox.__RANKI_WEBKIT1_VLIST_PATCH__ = true;
var htmlWebkit1 = katex.renderToString('x_1 + \\frac{a}{b}', { output: 'html', throwOnError: true });
if (htmlWebkit1.indexOf('vlist-t2') === -1 || htmlWebkit1.indexOf('vertical-align:') === -1) {
    console.error('[FAIL] WebKit 1.0 vlist-t2 patch did not inject vertical-align when enabled!');
    process.exit(1);
}
if (htmlModern === htmlWebkit1) {
    console.error('[FAIL] Conditional vlist-t2 patch was not gated properly between modern and WebKit 1.0!');
    process.exit(1);
}
console.log('[PASS] Conditional WebKit 1.0 vlist-t2 baseline patch verified.');

// 2. Test chemistry, bbox, cases, and higher math expressions
var specialCases = [
    '\\ce{MnO4- > Cl2}',
    '\\ce{2H2 + O2 -> 2H2O}',
    '\\ce{Fe^{2+} + 2OH- -> Fe(OH)2 v}',
    '\\ce{CO2 + C ->[\\Delta] 2CO}',
    '\\ce{Ca^2+}',
    '\\bbox[yellow,5px,border:2px solid red]{E = mc^2}',
    '\\begin{cases} x = \\varphi(t) \\\\ y = \\psi(t) \\end{cases}',
    '(uv)^{(n)} = \\sum_{k=0}^{n} \\mathrm{C}_n^k u^{(n-k)} v^{(k)}',
    '\\frac{\\mathrm{d}^2y}{\\mathrm{d}x^2} = \\frac{\\psi\'\'(t)\\varphi\'(t) - \\psi\'(t)\\varphi\'\'(t)}{[\\varphi\'(t)]^3}'
];
for (var s = 0; s < specialCases.length; s++) {
    katex.renderToString(specialCases[s], {
        output: 'html',
        throwOnError: true,
        macros: macros,
        strict: 'ignore'
    });
}
console.log('[PASS] All ' + specialCases.length + ' STEM/chemistry/bbox/calculus special cases passed.');

// 3. Test all 38 torture & real physics/chemistry/math benchmark cases (tools/test_cases.json)
var benchFile = path.join(projectRoot, 'tools', 'test_cases.json');
var benchCases = JSON.parse(fs.readFileSync(benchFile, 'utf8')).cases;
if (!benchCases || benchCases.length !== 38) {
    console.error('[FAIL] Expected 38 benchmark cases in ' + benchFile);
    process.exit(1);
}
var benchPass = 0;
var benchFailures = [];
var bt0 = Date.now();
for (var b = 0; b < benchCases.length; b++) {
    var bc = benchCases[b];
    try {
        katex.renderToString(bc.latex, {
            output: 'html',
            throwOnError: true,
            displayMode: !!bc.display,
            macros: macros,
            strict: 'ignore'
        });
        benchPass++;
    } catch (e) {
        benchFailures.push({ id: bc.id, name: bc.name, error: e.message });
    }
}
var benchMs = Date.now() - bt0;
console.log('38-Case Torture/STEM Benchmark Result: ' + benchPass + '/' + benchCases.length + ' passed in ' + benchMs + 'ms.');
if (benchFailures.length > 0) {
    console.error('[FAIL] Benchmark failures:', JSON.stringify(benchFailures, null, 2));
    process.exit(1);
}

// 4. Test all 838 real user formulas
var formulasFile = path.join(projectRoot, 'tools', 'fixtures', 'collection_formulas.json');
var formulasToTest = JSON.parse(fs.readFileSync(formulasFile, 'utf8')).formulas;
if (!formulasToTest || formulasToTest.length === 0) {
    console.error('[FAIL] No formulas loaded from ' + formulasFile);
    process.exit(1);
}

var passCount = 0;
var failCount = 0;
var failures = [];
var t0 = Date.now();
for (var i = 0; i < formulasToTest.length; i++) {
    var item = formulasToTest[i];
    var isDisplay = item.delimiter === '\\[' || item.delimiter === '$$';
    try {
        katex.renderToString(item.expr, {
            output: 'html',
            throwOnError: true,
            displayMode: isDisplay,
            macros: macros,
            strict: 'ignore'
        });
        passCount++;
    } catch (e) {
        failCount++;
        failures.push({ index: i, expr: item.expr, error: e.message });
    }
}
var elapsed = Date.now() - t0;
console.log('Formula Regression Result: ' + passCount + '/' + formulasToTest.length + ' passed in ' + elapsed + 'ms (' + failCount + ' failed).');

if (failCount > 0) {
    console.error('[FAIL] Formula regressions:', JSON.stringify(failures, null, 2));
    process.exit(1);
}

console.log('=== ALL REGRESSION TESTS PASSED (100%) ===');
