#!/usr/bin/env node
/*
 * tools/test_card_engine.js
 * Automated unit tests for views/card.js (CardEngine state machine, Card-ID guard,
 * HTML preprocessor, discrete pagination, font clamping, wide-equation centering/reset,
 * rapid render cancellation, and WebKit 1.0 detection).
 */
var fs = require('fs');
var path = require('path');
var vm = require('vm');
var assert = require('assert');

var repoRoot = path.resolve(__dirname, '..');
var cardJsPath = path.join(repoRoot, 'views', 'card.js');
var cardJsCode = fs.readFileSync(cardJsPath, 'utf8');

console.log('=== Running CardEngine Unit Tests (views/card.js) ===');

function createMockEnv(userAgent) {
    var elements = {};
    var ids = [
        'slot-a-front', 'slot-a-back', 'slot-b-front', 'slot-b-back',
        'card-a-front', 'card-a-back', 'card-b-front', 'card-b-back',
        'slot-finished', 'card-note-style', 'card-font-override', 'finished-sub'
    ];
    for (var i = 0; i < ids.length; i++) {
        elements[ids[i]] = {
            id: ids[i],
            className: ids[i].indexOf('slot-') === 0 ? 'slot-viewport offscreen' : 'card kindle',
            innerHTML: '',
            textContent: '',
            scrollTop: 0,
            clientHeight: 1000,
            scrollHeight: 1000,
            clientWidth: 960,
            style: {},
            querySelectorAll: function () { return []; }
        };
    }

    var titles = [];
    var timers = [];
    var nextTimerId = 1;

    var doc = {
        readyState: 'complete',
        _title: '',
        get title() { return this._title; },
        set title(v) { this._title = v; titles.push(v); },
        getElementById: function (id) { return elements[id] || null; },
        addEventListener: function () {},
        createElement: function () { return { style: {} }; },
        getElementsByTagName: function () { return [{ appendChild: function () {} }]; }
    };

    var sandbox = {
        window: {},
        document: doc,
        navigator: { userAgent: userAgent || 'Mozilla/5.0 AppleWebKit/537.6 (KHTML, like Gecko) Kindle' },
        console: console,
        renderMathInElement: function (el) { el._mathRendered = true; },
        setTimeout: function (fn, delay) {
            var id = nextTimerId++;
            timers.push({ id: id, fn: fn, delay: delay, cleared: false });
            return id;
        },
        clearTimeout: function (id) {
            for (var t = 0; t < timers.length; t++) {
                if (timers[t].id === id) timers[t].cleared = true;
            }
        }
    };
    sandbox.window = sandbox;
    vm.createContext(sandbox);
    vm.runInContext(cardJsCode, sandbox);

    return {
        sandbox: sandbox,
        engine: sandbox.CardEngine,
        elements: elements,
        titles: titles,
        flushTimers: function () {
            var safety = 0;
            while (timers.length > 0 && safety < 20) {
                var batch = timers.splice(0, timers.length);
                for (var b = 0; b < batch.length; b++) {
                    if (!batch[b].cleared) batch[b].fn();
                }
                safety++;
            }
        }
    };
}

// Test 1: WebKit 1.0 UA detection vs Modern Chrome
var envWk1 = createMockEnv('Mozilla/5.0 (X11; Linux armv7l) AppleWebKit/537.6 (KHTML, like Gecko)');
assert.strictEqual(envWk1.sandbox.__RANKI_WEBKIT1_VLIST_PATCH__, true, 'Expected WebKit 1.0 patch enabled');
var envChrome = createMockEnv('Mozilla/5.0 (Windows NT 10.0) AppleWebKit/537.36 Chrome/130.0.0.0 Safari/537.36');
assert.strictEqual(envChrome.sandbox.__RANKI_WEBKIT1_VLIST_PATCH__, false, 'Expected WebKit 1.0 patch disabled on Chrome');
console.log('[PASS] Test 1: WebKit 1.0 vlist-t2 auto-detection works cleanly.');

// Test 2: HTML preprocessor (scripts, bare \ce, unified color normalization, media placeholders, anti-tofu)
var pp = envWk1.engine._preprocessCardHtml;
assert.strictEqual(pp('<script>alert(1)</script><b>Hi</b>'), '<b>Hi</b>');
assert.strictEqual(pp('Water \\ce{H2O} and \\(\\ce{Ca^2+}\\)'), 'Water \\(\\ce{H2O}\\) and \\(\\ce{Ca^2+}\\)');
assert.ok(pp('<span style="color: red">trap</span>').indexOf('class="eink-red"') !== -1);
assert.ok(pp('<span style="color: #0066cc">blue</span>').indexOf('class="eink-color"') !== -1);
assert.ok(pp('<font color="green">green</font>').indexOf('class="eink-color"') !== -1);
assert.ok(pp('<span style="background-color: yellow">hl</span>').indexOf('class="eink-mark"') !== -1);
assert.strictEqual(pp('<span style="color: #555555">gray</span>').indexOf('eink-color'), -1, 'Neutral gray must not get artificial underline');
assert.ok(pp('Listen [sound:test_01.mp3]').indexOf('[\u4e0d\u652f\u6301\u97f3\u9891]') !== -1);
assert.ok(pp('<video src="a.mp4"></video>').indexOf('[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]') !== -1);
assert.ok(pp('Figure: <img src="fig1.png">').indexOf('[\u4e0d\u652f\u6301\u56fe\u7247]') !== -1);
assert.strictEqual(pp('\u2068OK\u2069 \uD83C\uDF89 \u2705 \u274C \u26A0\uFE0F'), 'OK  [\u5bf9] [\u9519] [!]');
assert.strictEqual(pp('Plain Chinese & English card with no math'), 'Plain Chinese & English card with no math');
console.log('[PASS] Test 2: HTML preprocessor handles colors, media placeholders, and anti-tofu sanitization.');

// Test 3: Font clamping [28, 54] with default 42px
assert.strictEqual(envWk1.engine.getState().fontSizePx, 42);
envWk1.engine.setConfig({ font_size_px: 10 });
assert.strictEqual(envWk1.engine.getState().fontSizePx, 28);
envWk1.engine.setConfig({ font_size_px: 99 });
assert.strictEqual(envWk1.engine.getState().fontSizePx, 54);
envWk1.engine.setConfig({ font_size_px: 42 });
assert.strictEqual(envWk1.engine.getState().fontSizePx, 42);
console.log('[PASS] Test 3: Font size defaults to 42px and clamps strictly within [28, 54].');

// Test 4: Render immediate, instant flip (flushing queued back), prefetch, and Card-ID verified swap
envWk1.engine.renderImmediate({
    current: { card_id: 101, question_html: 'Q101', answer_html: 'A101', css: '.c101{}' },
    next: { card_id: 102, question_html: 'Q102', answer_html: 'A102', css: '.c102{}' }
});
assert.strictEqual(envWk1.engine.getState().activeCardId, 101);
assert.strictEqual(envWk1.elements['card-a-front'].innerHTML, 'Q101');
assert.strictEqual(envWk1.elements['card-note-style'].textContent, '.c101{}');

// Flip immediately BEFORE timers flush -> must synchronously flush Back render!
envWk1.engine.flipToAnswer();
assert.strictEqual(envWk1.engine.getState().isFlipped, true);
assert.strictEqual(envWk1.elements['card-a-back'].innerHTML, 'A101');
assert.strictEqual(envWk1.elements['slot-a-back'].className, 'slot-viewport');

// Flush prefetch timer so Card 102 finishes pre-rendering in Group B
envWk1.flushTimers();
assert.strictEqual(envWk1.engine.getState().prefetchedCardId, 102);
assert.strictEqual(envWk1.elements['card-b-front'].innerHTML, 'Q102');

// Card-ID mismatch guard: attempting to swap to 999 MUST fail safely!
var badSwap = envWk1.engine.swapToPrefetched({ expected_card_id: 999 });
assert.strictEqual(badSwap, false, 'Stale Card-ID swap must be rejected');
assert.strictEqual(envWk1.engine.getState().activeCardId, 101, 'Active card must remain untouched on mismatch');

// Exact Card-ID match: swap to 102 succeeds and updates CSS to .c102{}
var okSwap = envWk1.engine.swapToPrefetched({
    expected_card_id: 102,
    prefetch_next: { card_id: 103, question_html: 'Q103', answer_html: 'A103', css: '.c103{}' }
});
assert.strictEqual(okSwap, true);
assert.strictEqual(envWk1.engine.getState().activeGroup, 'B');
assert.strictEqual(envWk1.engine.getState().activeCardId, 102);
assert.strictEqual(envWk1.elements['card-note-style'].textContent, '.c102{}');
assert.strictEqual(envWk1.elements['slot-b-front'].className, 'slot-viewport');
console.log('[PASS] Test 4: Card-ID verified 4-slot pipeline prevents desync and updates card CSS.');

// Test 5: Discrete 85% viewport pagination (deterministic page stops, exact clamp)
var vpB = envWk1.elements['slot-b-front'];
vpB.clientHeight = 1000;
vpB.scrollHeight = 2200; // maxScroll = 1200, step = 850 -> 3 pages (0, 850, 1200)
vpB.scrollTop = 0;
var st1 = envWk1.engine.pageScroll(1);
assert.strictEqual(vpB.scrollTop, 850);
assert.strictEqual(st1.page, 2);
assert.strictEqual(st1.total, 3);
var st2 = envWk1.engine.pageScroll(1);
assert.strictEqual(vpB.scrollTop, 1200);
assert.strictEqual(st2.page, 3);
var st3 = envWk1.engine.pageScroll(-1);
assert.strictEqual(vpB.scrollTop, 850);
assert.strictEqual(st3.page, 2);
var st4 = envWk1.engine.pageScroll(-1);
assert.strictEqual(vpB.scrollTop, 0);
assert.strictEqual(st4.page, 1);
console.log('[PASS] Test 5: Discrete 85% viewport pagination clamps and hits deterministic stops.');

// Test 6: Wide-equation centering layout-collapse & clean reset when font shrinks
var eqNode = { scrollWidth: 1200, offsetHeight: 100, style: {} };
envWk1.engine._scaleEquationNode(eqNode, 876);
// ratio = floor(876/1200 * 1000)/1000 = 0.73 -> scaledW = 876, marginRight = -324px
assert.strictEqual(eqNode.style.transform, 'scale(0.73)');
assert.strictEqual(eqNode.style.transformOrigin, '0 0');
assert.strictEqual(eqNode.style.marginRight, '-324px');
assert.strictEqual(eqNode.style.marginBottom, '-27px');
// Now simulate user shrinking font size so eqNode fits (700 <= 876): must cleanly reset transform & margins!
eqNode.scrollWidth = 700;
envWk1.engine._scaleEquationNode(eqNode, 876);
assert.strictEqual(eqNode.style.transform, '');
assert.strictEqual(eqNode.style.marginRight, '');
assert.strictEqual(eqNode.style.marginBottom, '');
console.log('[PASS] Test 6: Wide-equation scaling collapses box for exact centering and resets cleanly on font resize.');

// Test 7: Rapid back-to-back renderImmediate cancels stale back/prefetch timers cleanly
envWk1.engine.renderImmediate({
    current: { card_id: 201, question_html: 'Q201', answer_html: 'A201', css: '' },
    next: { card_id: 202, question_html: 'Q202', answer_html: 'A202', css: '' }
});
envWk1.engine.renderImmediate({
    current: { card_id: 301, question_html: 'Q301', answer_html: 'A301', css: '' },
    next: { card_id: 302, question_html: 'Q302', answer_html: 'A302', css: '' }
});
envWk1.flushTimers();
assert.strictEqual(envWk1.engine.getState().activeCardId, 301);
assert.strictEqual(envWk1.engine.getState().prefetchedCardId, 302);
assert.strictEqual(envWk1.elements['card-a-back'].innerHTML, 'A301', 'Stale card 201 back must not overwrite 301');
console.log('[PASS] Test 7: Rapid renderImmediate cancels stale back/prefetch renders without race corruption.');

// Test 8: Finished screen
envWk1.engine.showFinished('Done today');
assert.strictEqual(envWk1.elements['slot-finished'].className, 'slot-viewport');
assert.strictEqual(envWk1.elements['finished-sub'].textContent, 'Done today');
console.log('[PASS] Test 8: Finished state renders cleanly.');

// Test 9 (Regression Guard): Never break KaTeX/mhchem transparent \vphantom{X} struts!
var cardCssCode = fs.readFileSync(path.join(repoRoot, 'views', 'card.css'), 'utf8');
var cssNoComments = cardCssCode.replace(/\/\*[\s\S]*?\*\//g, '');
assert.strictEqual(
    /\.katex\s+\*/.test(cssNoComments),
    false,
    'card.css must NEVER contain `.katex *` wildcard rules that override color:transparent on \\vphantom{X} struts!'
);
var phantomNode = { style: { color: 'transparent' } };
var rgbaZeroNode = { style: { color: 'rgba(0, 0, 0, 0)' } };
var coloredNode = { style: { color: 'yellow' } };
var mockMathCard = {
    querySelectorAll: function () { return [phantomNode, rgbaZeroNode, coloredNode]; }
};
envWk1.engine._normalizeMathColors(mockMathCard);
assert.strictEqual(phantomNode.style.color, 'transparent', '\\vphantom transparent strut must stay transparent');
assert.strictEqual(rgbaZeroNode.style.color, 'rgba(0, 0, 0, 0)', 'rgba(0,0,0,0) strut must stay transparent');
assert.strictEqual(coloredNode.style.color, '#000000', 'Colored TeX text must normalize to pure black #000000');
console.log('[PASS] Test 9: KaTeX/mhchem transparent \\vphantom struts are protected while colored TeX normalizes to #000000.');

console.log('=== ALL 9 CARD ENGINE UNIT TESTS PASSED ===');
