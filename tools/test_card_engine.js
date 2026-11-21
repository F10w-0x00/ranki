#!/usr/bin/env node
/*
 * tools/test_card_engine.js
 * Automated unit tests for views/card.js (CardEngine state machine, Card-ID guard,
 * HTML preprocessor, discrete pagination, font clamping, and WebKit 1.0 detection).
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

// Test 2: HTML preprocessor (strip scripts, wrap bare \ce, protect math, normalize red)
var pp = envWk1.engine._preprocessCardHtml;
assert.strictEqual(pp('<script>alert(1)</script><b>Hi</b>'), '<b>Hi</b>');
assert.strictEqual(pp('Water \\ce{H2O} and \\(\\ce{Ca^2+}\\)'), 'Water \\(\\ce{H2O}\\) and \\(\\ce{Ca^2+}\\)');
assert.ok(pp('<span style="color: red">trap</span>').indexOf('class="eink-red"') !== -1);
console.log('[PASS] Test 2: HTML preprocessor sanitizes scripts, wraps bare \\ce, preserves math, normalizes red.');

// Test 3: Font clamping
envWk1.engine.setConfig({ font_size_px: 10 });
assert.strictEqual(envWk1.engine.getState().fontSizePx, 20);
envWk1.engine.setConfig({ font_size_px: 99 });
assert.strictEqual(envWk1.engine.getState().fontSizePx, 38);
envWk1.engine.setConfig({ font_size_px: 28 });
assert.strictEqual(envWk1.engine.getState().fontSizePx, 28);
console.log('[PASS] Test 3: Font size clamps strictly within [20, 38].');

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

// Test 5: Discrete 85% viewport pagination (zero paging bar, exact clamp)
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
console.log('[PASS] Test 5: Discrete 85% viewport pagination clamps and computes page/total accurately.');

// Test 6: Finished screen
envWk1.engine.showFinished('Done today');
assert.strictEqual(envWk1.elements['slot-finished'].className, 'slot-viewport');
assert.strictEqual(envWk1.elements['finished-sub'].textContent, 'Done today');
console.log('[PASS] Test 6: Finished state renders cleanly.');

console.log('=== ALL 6 CARD ENGINE UNIT TESTS PASSED ===');
