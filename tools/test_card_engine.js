#!/usr/bin/env node
/*
 * tools/test_card_engine.js
 * Automated unit tests for views/card.js (pure MathJax 2.7.9 CardEngine state machine,
 * Card-ID guard, HTML preprocessor, discrete pagination, font clamping,
 * <li> wide-equation explicit layout-box wrapper & reset, atomic single-frame reveal,
 * and monochrome MathJax \bbox normalization).
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
        'slot-finished', 'card-note-style', 'card-font-override', 'finished-title', 'finished-sub'
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
    var visibilitySnapshots = [];

    var doc = {
        readyState: 'complete',
        _title: '',
        get title() { return this._title; },
        set title(v) { this._title = v; titles.push(v); },
        getElementById: function (id) { return elements[id] || null; },
        addEventListener: function () {},
        createElement: function (tag) {
            return {
                tagName: String(tag || 'span').toUpperCase(),
                className: '',
                style: {},
                children: [],
                parentNode: null,
                appendChild: function (child) {
                    child.parentNode = this;
                    this.children.push(child);
                    return child;
                }
            };
        },
        getElementsByTagName: function () { return [{ appendChild: function () {} }]; }
    };

    var sandbox = {
        window: {},
        document: doc,
        navigator: { userAgent: userAgent || 'Mozilla/5.0 AppleWebKit/537.6 (KHTML, like Gecko) Kindle' },
        console: console,
        MathJax: {
            isReady: true,
            Hub: {
                Register: {
                    StartupHook: function (_evt, fn) { fn(); }
                },
                Queue: function (task, cb) {
                    if (task && task[2]) {
                        visibilitySnapshots.push(task[2].style.visibility || '');
                        task[2]._mathJaxRendered = true;
                    }
                    if (typeof cb === 'function') cb();
                }
            }
        },
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
        visibilitySnapshots: visibilitySnapshots,
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

var env = createMockEnv();

// Test 1: Pure MathJax 2.7.9 engine and CARD_READY signal
assert.strictEqual(env.engine.getState().mathEngine, 'mathjax', 'Engine must be pure MathJax 2.7.9');
assert.ok(env.titles[0] && env.titles[0].indexOf('RA:CARD_READY:') === 0, 'CARD_READY must fire on init');
console.log('[PASS] Test 1: Pure MathJax 2.7.9 CardEngine initializes and emits CARD_READY.');

// Test 2: HTML preprocessor (scripts, bare \ce, unified color normalization, bilingual media placeholders, anti-tofu)
assert.strictEqual(env.engine.getState().uiLang, 'en', 'Default UI language must be English');
var pp = env.engine._preprocessCardHtml;
assert.strictEqual(pp('<script>alert(1)</script><b>Hi</b>'), '<b>Hi</b>');
assert.strictEqual(pp('Water \\ce{H2O} and \\(\\ce{Ca^2+}\\)'), 'Water \\(\\ce{H2O}\\) and \\(\\ce{Ca^2+}\\)');
assert.ok(pp('<span style="color: red">trap</span>').indexOf('class="eink-red"') !== -1);
assert.ok(pp('<span style="color: #0066cc">blue</span>').indexOf('class="eink-color"') !== -1);
assert.ok(pp('<font color="green">green</font>').indexOf('class="eink-color"') !== -1);
assert.ok(pp('<span style="background-color: yellow">hl</span>').indexOf('class="eink-mark"') !== -1);
assert.strictEqual(pp('<span style="color: #555555">gray</span>').indexOf('eink-color'), -1, 'Neutral gray must not get artificial underline');
assert.ok(pp('Listen [sound:test_01.mp3]').indexOf('[Audio unsupported]') !== -1);
assert.ok(pp('<video src="a.mp4"></video>').indexOf('[Video/Embed unsupported]') !== -1);
assert.ok(pp('Figure: <img src="fig1.png">').indexOf('[Image unsupported]') !== -1);
assert.strictEqual(pp('\u2068OK\u2069 \uD83C\uDF89 \u2705 \u274C \u26A0\uFE0F'), 'OK  [OK] [X] [!]');
// Switch to Chinese and verify localized placeholders & finished title
env.engine.setConfig({ lang: 'zh' });
assert.strictEqual(env.engine.getState().uiLang, 'zh');
assert.ok(pp('Listen [sound:test_01.mp3]').indexOf('[\u4e0d\u652f\u6301\u97f3\u9891]') !== -1);
assert.ok(pp('<video src="a.mp4"></video>').indexOf('[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]') !== -1);
assert.ok(pp('Figure: <img src="fig1.png">').indexOf('[\u4e0d\u652f\u6301\u56fe\u7247]') !== -1);
assert.strictEqual(pp('\u2068OK\u2069 \uD83C\uDF89 \u2705 \u274C \u26A0\uFE0F'), 'OK  [\u5bf9] [\u9519] [!]');
assert.strictEqual(env.elements['finished-title'].textContent, '\u4eca\u65e5\u590d\u4e60\u5df2\u5b8c\u6210');
env.engine.setConfig({ lang: 'en' });
assert.strictEqual(env.elements['finished-title'].textContent, 'Congratulations!');
assert.strictEqual(pp('Plain Chinese & English card with no math'), 'Plain Chinese & English card with no math');
console.log('[PASS] Test 2: HTML preprocessor handles colors, bilingual placeholders, and anti-tofu sanitization.');

// Test 3: Font size defaults to 42px and clamps strictly within [28, 54]
assert.strictEqual(env.engine.getState().fontSizePx, 42);
env.engine.setConfig({ font_size_px: 10 });
assert.strictEqual(env.engine.getState().fontSizePx, 28);
env.engine.setConfig({ font_size_px: 99 });
assert.strictEqual(env.engine.getState().fontSizePx, 54);
env.engine.setConfig({ font_size_px: 42 });
assert.strictEqual(env.engine.getState().fontSizePx, 42);
console.log('[PASS] Test 3: Font size defaults to 42px and clamps strictly within [28, 54].');

// Test 4: Atomic single-frame reveal + Card-ID verified 4-slot pipeline
env.engine.renderImmediate({
    current: { card_id: 101, question_html: 'Q101', answer_html: 'A101', css: '.c101{}' },
    next: { card_id: 102, question_html: 'Q102', answer_html: 'A102', css: '.c102{}' }
});
assert.strictEqual(env.visibilitySnapshots[0], 'hidden', 'Front must stay hidden while MathJax typesets to avoid 2-frame ghosting');
assert.strictEqual(env.elements['card-a-front'].style.visibility, 'visible', 'Front must become visible once MathJax finishes');
assert.strictEqual(env.engine.getState().activeCardId, 101);
assert.strictEqual(env.elements['card-a-front'].innerHTML, 'Q101');
assert.strictEqual(env.elements['card-note-style'].textContent, '.c101{}');

env.engine.flipToAnswer();
assert.strictEqual(env.engine.getState().isFlipped, true);
assert.strictEqual(env.elements['card-a-back'].innerHTML, 'A101');
assert.strictEqual(env.elements['slot-a-back'].className, 'slot-viewport');

env.flushTimers();
assert.strictEqual(env.engine.getState().prefetchedCardId, 102);
assert.strictEqual(env.elements['card-b-front'].innerHTML, 'Q102');

var badSwap = env.engine.swapToPrefetched({ expected_card_id: 999 });
assert.strictEqual(badSwap, false, 'Stale Card-ID swap must be rejected');
assert.strictEqual(env.engine.getState().activeCardId, 101, 'Active card must remain untouched on mismatch');

var okSwap = env.engine.swapToPrefetched({
    expected_card_id: 102,
    prefetch_next: { card_id: 103, question_html: 'Q103', answer_html: 'A103', css: '.c103{}' }
});
assert.strictEqual(okSwap, true);
assert.strictEqual(env.engine.getState().activeGroup, 'B');
assert.strictEqual(env.engine.getState().activeCardId, 102);
assert.strictEqual(env.elements['card-note-style'].textContent, '.c102{}');
assert.strictEqual(env.elements['slot-b-front'].className, 'slot-viewport');
console.log('[PASS] Test 4: Atomic single-frame reveal & Card-ID verified 4-slot pipeline pass.');

// Test 5: Discrete 85% viewport pagination (deterministic page stops, exact clamp)
var vpB = env.elements['slot-b-front'];
vpB.clientHeight = 1000;
vpB.scrollHeight = 2200;
vpB.scrollTop = 0;
var st1 = env.engine.pageScroll(1);
assert.strictEqual(vpB.scrollTop, 850);
assert.strictEqual(st1.page, 2);
assert.strictEqual(st1.total, 3);
var st2 = env.engine.pageScroll(1);
assert.strictEqual(vpB.scrollTop, 1200);
assert.strictEqual(st2.page, 3);
var st3 = env.engine.pageScroll(-1);
assert.strictEqual(vpB.scrollTop, 850);
assert.strictEqual(st3.page, 2);
var st4 = env.engine.pageScroll(-1);
assert.strictEqual(vpB.scrollTop, 0);
assert.strictEqual(st4.page, 1);
console.log('[PASS] Test 5: Discrete 85% viewport pagination clamps and hits deterministic stops.');

// Test 6: Wide-equation <li> container width cap + explicit .eq-scale-wrap box & clean unwrap
var mockCard = { tagName: 'DIV', clientWidth: 960 };
var mockLi = {
    tagName: 'LI',
    parentNode: mockCard,
    children: [],
    insertBefore: function (newNode, refNode) {
        newNode.parentNode = this;
        var idx = this.children.indexOf(refNode);
        if (idx === -1) this.children.push(newNode);
        else this.children.splice(idx, 0, newNode);
    },
    removeChild: function (oldNode) {
        var idx = this.children.indexOf(oldNode);
        if (idx !== -1) this.children.splice(idx, 1);
        oldNode.parentNode = null;
    }
};
var eqInLi = { tagName: 'SPAN', scrollWidth: 850, offsetHeight: 54, style: {}, parentNode: mockLi };
mockLi.children.push(eqInLi);

var liAvail = env.engine._getAvailableContainerWidth(eqInLi, mockCard, 876);
assert.ok(liAvail < 820 && liAvail > 760, 'Expected <li> width cap around ~802px, got ' + liAvail);

env.engine._scaleEquationNode(eqInLi, liAvail);
assert.strictEqual(eqInLi.parentNode.className, 'eq-scale-wrap', 'Wide equation in DOM must be wrapped in explicit .eq-scale-wrap');
assert.strictEqual(eqInLi.style.transformOrigin, '0 0');
assert.ok(parseInt(eqInLi.parentNode.style.width, 10) <= liAvail, 'Wrapper width must fit inside <li>');

// Shrink font so equation fits naturally -> must unwrap .eq-scale-wrap cleanly!
eqInLi.scrollWidth = 650;
env.engine._scaleEquationNode(eqInLi, liAvail);
assert.strictEqual(eqInLi.parentNode, mockLi, 'Unscaled equation must unwrap back into <li>');
assert.strictEqual(eqInLi.style.transform, '');
console.log('[PASS] Test 6: <li> container cap & explicit .eq-scale-wrap prevent bullet wrapping and unwrap cleanly.');

// Test 7: Rapid back-to-back renderImmediate cancels stale back/prefetch timers cleanly
env.engine.renderImmediate({
    current: { card_id: 201, question_html: 'Q201', answer_html: 'A201', css: '' },
    next: { card_id: 202, question_html: 'Q202', answer_html: 'A202', css: '' }
});
env.engine.renderImmediate({
    current: { card_id: 301, question_html: 'Q301', answer_html: 'A301', css: '' },
    next: { card_id: 302, question_html: 'Q302', answer_html: 'A302', css: '' }
});
env.flushTimers();
assert.strictEqual(env.engine.getState().activeCardId, 301);
assert.strictEqual(env.engine.getState().prefetchedCardId, 302);
assert.strictEqual(env.elements['card-a-back'].innerHTML, 'A301', 'Stale card 201 back must not overwrite 301');
console.log('[PASS] Test 7: Rapid renderImmediate cancels stale back/prefetch renders without race corruption.');

// Test 8: Finished screen
env.engine.showFinished('Done today');
assert.strictEqual(env.elements['slot-finished'].className, 'slot-viewport');
assert.strictEqual(env.elements['finished-sub'].textContent, 'Done today');
console.log('[PASS] Test 8: Finished state renders cleanly.');

// Test 9: Monochrome MathJax \bbox & color normalization strictly protects transparent layout struts
var cardCssCode = fs.readFileSync(path.join(repoRoot, 'views', 'card.css'), 'utf8');
var cssNoComments = cardCssCode.replace(/\/\*[\s\S]*?\*\//g, '');
assert.strictEqual(
    /\.MathJax\s+\*/.test(cssNoComments),
    false,
    'card.css must NEVER contain `.MathJax *` wildcard rules that override transparent layout struts!'
);
var phantomNode = { style: { color: 'transparent', borderColor: 'transparent' } };
var rgbaZeroNode = { style: { color: 'rgba(0, 0, 0, 0)' } };
var coloredNode = { style: { color: 'yellow', backgroundColor: 'yellow', borderColor: 'red' } };
var mockMathCard = {
    querySelectorAll: function () { return [phantomNode, rgbaZeroNode, coloredNode]; }
};
env.engine._normalizeMathColors(mockMathCard);
assert.strictEqual(phantomNode.style.color, 'transparent');
assert.strictEqual(phantomNode.style.borderColor, 'transparent');
assert.strictEqual(rgbaZeroNode.style.color, 'rgba(0, 0, 0, 0)');
assert.strictEqual(coloredNode.style.color, '#000000');
assert.strictEqual(coloredNode.style.backgroundColor, '#e6e6e6');
assert.strictEqual(coloredNode.style.borderColor, '#000000');
console.log('[PASS] Test 9: MathJax monochrome normalization protects transparent struts and converts colored \\bbox.');

console.log('=== ALL 9 CARD ENGINE UNIT TESTS PASSED ===');
