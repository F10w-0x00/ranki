/* RAnki Route B — Resident Card Canvas Engine (views/card.js)
 * Strict ES5 for WebKitGTK 1.0 (JavaScriptCore 2012)
 * Zero ES6+, zero post-2012 DOM APIs, zero Emoji, zero localStorage.
 */
(function () {
    // 1. Auto-enable WebKitGTK 1.0 (2012) KaTeX vlist-t2 baseline fix only on old WebKit
    var ua = (typeof navigator !== 'undefined' && navigator.userAgent) ? navigator.userAgent : '';
    var wkMatch = ua.match(/AppleWebKit\/(\d+)/);
    var isChromium = (ua.indexOf('Chrome/') !== -1 || ua.indexOf('Edg/') !== -1);
    window.__RANKI_WEBKIT1_VLIST_PATCH__ = !!(!isChromium && wkMatch && parseInt(wkMatch[1], 10) < 600);

    // 2. Engine State
    var tokenCounter = 1;
    var activeGroup = 'A';       // 'A' or 'B'
    var isFlipped = false;
    var activeCardId = 0;
    var prefetchedCardId = 0;
    var activeCardCss = '';
    var prefetchedCardCss = '';

    var fontSizePx = 28;
    var mathEngine = 'katex';    // 'katex' or 'mathjax'
    var mathJaxLoaded = false;
    var mathJaxLoading = false;
    var mathJaxWaiters = [];

    var pendingBackRender = null;
    var pendingPrefetchTimer = null;

    // DOM References
    var slotAFront = null;
    var slotABack = null;
    var slotBFront = null;
    var slotBBack = null;
    var cardAFront = null;
    var cardABack = null;
    var cardBFront = null;
    var cardBBack = null;
    var slotFinished = null;
    var noteStyleEl = null;
    var fontOverrideEl = null;

    var KATEX_MACROS = {
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

    var KATEX_DELIMITERS = [
        { left: '$$', right: '$$', display: true },
        { left: '\\[', right: '\\]', display: true },
        { left: '\\(', right: '\\)', display: false },
        { left: '$', right: '$', display: false }
    ];

    function nowMs() {
        return new Date().getTime();
    }

    // 3. Host Up-Channel (monotonic token via document.title + preview iframe postMessage)
    function notifyHost(action, payload) {
        var token = tokenCounter++;
        var json = JSON.stringify(payload || {});
        if (typeof document !== 'undefined') {
            document.title = 'RA:' + action + ':' + token + ':' + json;
        }
        try {
            if (typeof window !== 'undefined' && window.parent && window.parent !== window && window.parent.postMessage) {
                window.parent.postMessage({ source: 'ranki-card', action: action, token: token, payload: payload || {} }, '*');
            }
        } catch (e) {
            // Ignore cross-origin frame errors in standalone preview
        }
    }

    // 4. Card HTML Preprocessor (pure ES5, safe & deterministic)
    function preprocessCardHtml(html) {
        if (!html) return '';

        // Strip <script> blocks defensively
        var clean = String(html).replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');

        // Protect existing math delimiters before scanning for bare \ce{...}
        var mathBlocks = [];
        var protectedHtml = clean.replace(/(\$\$[\s\S]*?\$\$|\\\[[\s\S]*?\\\]|\$[\s\S]*?\$|\\\([\s\S]*?\\\))/g, function (match) {
            mathBlocks.push(match);
            return '___RANKI_MATH_' + (mathBlocks.length - 1) + '___';
        });

        // Wrap standalone \ce{...} in inline math delimiters
        protectedHtml = protectedHtml.replace(/(\\ce\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\})/g, function (match) {
            return '\\(' + match + '\\)';
        });

        // Normalize red-colored spans/fonts to monochrome emphasis class
        protectedHtml = protectedHtml.replace(
            /(<(?:span|font)\b[^>]*style\s*=\s*["'][^"']*color\s*:\s*(?:red|#ff0000|rgb\(\s*255\s*,\s*0\s*,\s*0\s*\))[^"']*["'][^>]*)(>)/gi,
            function (all, openTag, closeBracket) {
                if (openTag.indexOf('class=') !== -1) {
                    return openTag.replace(/class=(["'])/i, 'class=$1eink-red ') + closeBracket;
                }
                return openTag + ' class="eink-red"' + closeBracket;
            }
        );

        // Restore protected math blocks
        var restored = protectedHtml.replace(/___RANKI_MATH_(\d+)___/g, function (match, idx) {
            return mathBlocks[parseInt(idx, 10)];
        });

        return restored;
    }

    // 5. Font Size & Dynamic Card Style Management
    function applyFontSize(px) {
        var clamped = parseInt(px, 10);
        if (isNaN(clamped)) clamped = 28;
        if (clamped < 20) clamped = 20;
        if (clamped > 38) clamped = 38;
        fontSizePx = clamped;

        if (fontOverrideEl) {
            fontOverrideEl.textContent =
                '.card.kindle { font-size: ' + fontSizePx + 'px !important; }';
        }
    }

    function applyNoteCss(css) {
        if (noteStyleEl) {
            noteStyleEl.textContent = css || '';
        }
    }

    // 6. Auto-scale wide equations via -webkit-transform (zero horizontal scrollbars)
    function fitWideEquations(cardEl) {
        if (!cardEl || !cardEl.querySelectorAll) return;
        var maxW = cardEl.clientWidth - 96; // account for 48px horizontal padding each side
        if (maxW <= 100) maxW = 860;

        var nodes = cardEl.querySelectorAll('.katex-display, .katex, .MathJax_Display');
        for (var i = 0; i < nodes.length; i++) {
            var el = nodes[i];
            // Skip inline .katex inside a .katex-display parent (parent handles scaling)
            if (el.className && el.className.indexOf('katex-display') === -1 &&
                el.parentNode && el.parentNode.className &&
                el.parentNode.className.indexOf('katex-display') !== -1) {
                continue;
            }
            var sw = el.scrollWidth || 0;
            if (sw > maxW && sw > 0) {
                var ratio = Math.floor((maxW / sw) * 1000) / 1000;
                if (ratio < 0.45) ratio = 0.45;
                el.style.webkitTransform = 'scale(' + ratio + ')';
                el.style.transform = 'scale(' + ratio + ')';
                el.style.webkitTransformOrigin = 'center top';
                el.style.transformOrigin = 'center top';
                el.style.display = 'inline-block';
            }
        }
    }

    // 7. Lazy MathJax 2.7.9 Fallback Loader (only loaded when mathEngine === 'mathjax')
    function ensureMathJaxLoaded(cb) {
        if (mathJaxLoaded && window.MathJax && window.MathJax.Hub) {
            cb();
            return;
        }
        mathJaxWaiters.push(cb);
        if (mathJaxLoading) return;
        mathJaxLoading = true;

        window.MathJax = {
            showProcessingMessages: false,
            messageStyle: 'none',
            skipStartupTypeset: true,
            extensions: ['tex2jax.js'],
            jax: ['input/TeX', 'output/HTML-CSS'],
            tex2jax: {
                inlineMath: [['$', '$'], ['\\(', '\\)']],
                displayMath: [['$$', '$$'], ['\\[', '\\]']],
                processEscapes: true
            },
            TeX: {
                extensions: [
                    'AMSmath.js', 'AMSsymbols.js', 'mhchem.js', 'extpfeil.js',
                    'color.js', 'bbox.js', 'newcommand.js', 'noErrors.js', 'noUndefined.js'
                ]
            },
            'HTML-CSS': {
                scale: 108,
                availableFonts: ['TeX'],
                preferredFont: 'TeX',
                webFont: 'TeX',
                imageFont: null,
                matchFontHeight: false
            },
            AuthorInit: function () {
                window.MathJax.Hub.Register.StartupHook('End', function () {
                    mathJaxLoaded = true;
                    mathJaxLoading = false;
                    var list = mathJaxWaiters.slice(0);
                    mathJaxWaiters = [];
                    for (var i = 0; i < list.length; i++) {
                        list[i]();
                    }
                });
            }
        };

        var script = document.createElement('script');
        script.type = 'text/javascript';
        script.src = 'mathjax/MathJax.js?config=TeX-AMS_HTML';
        document.getElementsByTagName('head')[0].appendChild(script);
    }

    // 8. Core Typesetting Function (KaTeX synchronous ~20-40ms or MathJax fallback)
    function renderIntoElement(cardEl, rawHtml, onDone) {
        var t0 = nowMs();
        cardEl.innerHTML = preprocessCardHtml(rawHtml);

        if (mathEngine === 'mathjax') {
            ensureMathJaxLoaded(function () {
                window.MathJax.Hub.Queue(['Typeset', window.MathJax.Hub, cardEl], function () {
                    fitWideEquations(cardEl);
                    if (onDone) onDone(nowMs() - t0);
                });
            });
            return;
        }

        // Primary: Ultra-Fast ES5 KaTeX synchronous rendering
        if (typeof window.renderMathInElement === 'function') {
            try {
                window.renderMathInElement(cardEl, {
                    delimiters: KATEX_DELIMITERS,
                    macros: KATEX_MACROS,
                    strict: 'ignore',
                    throwOnError: false
                });
            } catch (e) {
                if (typeof console !== 'undefined' && console.log) {
                    console.log('[CardEngine] KaTeX render warning: ' + e.message);
                }
            }
        }
        fitWideEquations(cardEl);
        if (onDone) onDone(nowMs() - t0);
    }

    // 9. Slot Accessors & Visibility Helpers
    function getGroupSlots(group) {
        if (group === 'A') {
            return {
                frontView: slotAFront, backView: slotABack,
                frontCard: cardAFront, backCard: cardABack
            };
        }
        return {
            frontView: slotBFront, backView: slotBBack,
            frontCard: cardBFront, backCard: cardBBack
        };
    }

    function getActiveViewport() {
        var g = getGroupSlots(activeGroup);
        return isFlipped ? g.backView : g.frontView;
    }

    function setVisibleSlot(targetView) {
        var all = [slotAFront, slotABack, slotBFront, slotBBack, slotFinished];
        for (var i = 0; i < all.length; i++) {
            if (!all[i]) continue;
            if (all[i] === targetView) {
                all[i].className = 'slot-viewport';
            } else {
                all[i].className = 'slot-viewport offscreen';
            }
        }
    }

    // 10. Discrete Zero-Bar Pagination (deterministic page stops, 0 intermediate drag frames)
    var currentPageIndex = 0;

    function getPaginationMetrics(vp) {
        if (!vp) return { step: 850, maxScroll: 0, total: 1 };
        var h = vp.clientHeight || 1200;
        var sh = vp.scrollHeight || h;
        if (sh <= h + 16) {
            return { step: h, maxScroll: 0, total: 1 };
        }
        var step = Math.max(120, Math.floor(h * 0.85));
        var maxScroll = Math.max(0, sh - h);
        var total = Math.floor((maxScroll + step - 1) / step) + 1;
        return { step: step, maxScroll: maxScroll, total: total };
    }

    function computePageState(vp) {
        var m = getPaginationMetrics(vp);
        if (m.total <= 1) {
            currentPageIndex = 0;
            return { page: 1, total: 1 };
        }
        if (currentPageIndex < 0) currentPageIndex = 0;
        if (currentPageIndex >= m.total) currentPageIndex = m.total - 1;
        return { page: currentPageIndex + 1, total: m.total };
    }

    function resetPageIndex(vp) {
        currentPageIndex = 0;
        if (vp) vp.scrollTop = 0;
    }

    function emitPageState() {
        var vp = getActiveViewport();
        var st = computePageState(vp);
        notifyHost('PAGE_STATE', st);
        return st;
    }

    function pageScroll(direction) {
        var vp = getActiveViewport();
        if (!vp) return { page: 1, total: 1 };
        var m = getPaginationMetrics(vp);
        if (m.total <= 1) {
            currentPageIndex = 0;
            return { page: 1, total: 1 };
        }
        var nextIdx = currentPageIndex + (direction > 0 ? 1 : -1);
        if (nextIdx < 0) nextIdx = 0;
        if (nextIdx >= m.total) nextIdx = m.total - 1;
        currentPageIndex = nextIdx;
        var targetTop = (nextIdx === m.total - 1) ? m.maxScroll : (nextIdx * m.step);
        if (targetTop > m.maxScroll) targetTop = m.maxScroll;
        vp.scrollTop = targetTop;
        return emitPageState();
    }

    // 11. Background Prefetch Helper
    function cancelBackgroundWork() {
        if (pendingPrefetchTimer) {
            clearTimeout(pendingPrefetchTimer);
            pendingPrefetchTimer = null;
        }
    }

    function schedulePrefetch(nextCard) {
        cancelBackgroundWork();
        var inactiveGroup = (activeGroup === 'A') ? 'B' : 'A';
        var bg = getGroupSlots(inactiveGroup);

        if (!nextCard) {
            prefetchedCardId = 0;
            prefetchedCardCss = '';
            if (bg.frontCard) bg.frontCard.innerHTML = '';
            if (bg.backCard) bg.backCard.innerHTML = '';
            return;
        }

        prefetchedCardId = 0; // Only mark ready after both sides finish rendering
        prefetchedCardCss = nextCard.css || '';

        pendingPrefetchTimer = setTimeout(function () {
            pendingPrefetchTimer = null;
            renderIntoElement(bg.frontCard, nextCard.question_html || '', function () {
                renderIntoElement(bg.backCard, nextCard.answer_html || '', function () {
                    prefetchedCardId = nextCard.card_id || 0;
                });
            });
        }, 30);
    }

    function flushPendingBack() {
        if (pendingBackRender) {
            var fn = pendingBackRender;
            pendingBackRender = null;
            fn();
        }
    }

    // 12. Public CardEngine API called by Rust Host (or Preview Harness)
    var CardEngine = {
        _preprocessCardHtml: preprocessCardHtml,
        _computePageState: computePageState,

        setConfig: function (cfg) {
            if (!cfg) return;
            if (typeof cfg.font_size_px === 'number') {
                applyFontSize(cfg.font_size_px);
            }
            if (cfg.math_engine === 'katex' || cfg.math_engine === 'mathjax') {
                mathEngine = cfg.math_engine;
            }
            // Refit active viewport & report updated pagination after font change
            var vp = getActiveViewport();
            if (vp && vp.className === 'slot-viewport') {
                var g = getGroupSlots(activeGroup);
                fitWideEquations(isFlipped ? g.backCard : g.frontCard);
                emitPageState();
            }
        },

        renderImmediate: function (payload) {
            cancelBackgroundWork();
            pendingBackRender = null;

            if (!payload || !payload.current) {
                CardEngine.showFinished(payload && payload.message);
                return;
            }

            var cur = payload.current;
            var next = payload.next || null;

            activeGroup = 'A';
            isFlipped = false;
            activeCardId = cur.card_id || 0;
            activeCardCss = cur.css || '';
            applyNoteCss(activeCardCss);

            var g = getGroupSlots('A');
            resetPageIndex(g.frontView);
            g.backView.scrollTop = 0;
            setVisibleSlot(g.frontView);

            // Render visible Front immediately
            renderIntoElement(g.frontCard, cur.question_html || '', function (elapsedMs) {
                var st = emitPageState();
                notifyHost('RENDER_DONE', {
                    card_id: activeCardId,
                    mode: 'immediate',
                    ms: elapsedMs,
                    page: st.page,
                    total: st.total
                });

                // Render Back cleanly right after Front paints, then prefetch Next
                var backHtml = cur.answer_html || '';
                var backCid = activeCardId;
                pendingBackRender = function () {
                    if (activeCardId !== backCid) return;
                    renderIntoElement(g.backCard, backHtml, function () {
                        schedulePrefetch(next);
                    });
                };
                setTimeout(function () {
                    flushPendingBack();
                }, 15);
            });
        },

        flipToAnswer: function () {
            if (isFlipped || activeCardId === 0) return;
            var t0 = nowMs();
            flushPendingBack();
            isFlipped = true;
            var g = getGroupSlots(activeGroup);
            resetPageIndex(g.backView);
            setVisibleSlot(g.backView);
            var st = emitPageState();
            notifyHost('FLIP_DONE', {
                card_id: activeCardId,
                ms: nowMs() - t0,
                page: st.page,
                total: st.total
            });
        },

        swapToPrefetched: function (payload) {
            if (!payload) return false;
            var t0 = nowMs();
            var expectedId = payload.expected_card_id || 0;

            // Card-ID verification guard: refuse swap if prefetch is stale/not ready
            if (expectedId === 0 || prefetchedCardId !== expectedId) {
                if (payload.fallback_current) {
                    CardEngine.renderImmediate({
                        current: payload.fallback_current,
                        next: payload.prefetch_next || null
                    });
                    return true;
                }
                notifyHost('PREFETCH_MISS', { expected_card_id: expectedId, prefetched_card_id: prefetchedCardId });
                return false;
            }

            cancelBackgroundWork();
            pendingBackRender = null;

            activeGroup = (activeGroup === 'A') ? 'B' : 'A';
            isFlipped = false;
            activeCardId = expectedId;
            activeCardCss = prefetchedCardCss;
            applyNoteCss(activeCardCss);

            var g = getGroupSlots(activeGroup);
            resetPageIndex(g.frontView);
            g.backView.scrollTop = 0;
            setVisibleSlot(g.frontView);

            var st = emitPageState();
            notifyHost('RENDER_DONE', {
                card_id: activeCardId,
                mode: 'prefetch_hit',
                ms: nowMs() - t0,
                page: st.page,
                total: st.total
            });

            schedulePrefetch(payload.prefetch_next || null);
            return true;
        },

        pageScroll: pageScroll,

        showFinished: function (subtitle) {
            cancelBackgroundWork();
            pendingBackRender = null;
            activeCardId = 0;
            prefetchedCardId = 0;
            isFlipped = false;
            var subEl = document.getElementById('finished-sub');
            if (subEl && subtitle) {
                subEl.textContent = subtitle;
            }
            setVisibleSlot(slotFinished);
            notifyHost('PAGE_STATE', { page: 1, total: 1 });
        },

        getState: function () {
            return {
                activeGroup: activeGroup,
                activeCardId: activeCardId,
                prefetchedCardId: prefetchedCardId,
                isFlipped: isFlipped,
                fontSizePx: fontSizePx,
                mathEngine: mathEngine
            };
        }
    };

    window.CardEngine = CardEngine;

    function initDom() {
        slotAFront = document.getElementById('slot-a-front');
        slotABack = document.getElementById('slot-a-back');
        slotBFront = document.getElementById('slot-b-front');
        slotBBack = document.getElementById('slot-b-back');
        cardAFront = document.getElementById('card-a-front');
        cardABack = document.getElementById('card-a-back');
        cardBFront = document.getElementById('card-b-front');
        cardBBack = document.getElementById('card-b-back');
        slotFinished = document.getElementById('slot-finished');
        noteStyleEl = document.getElementById('card-note-style');
        fontOverrideEl = document.getElementById('card-font-override');

        applyFontSize(fontSizePx);
        notifyHost('CARD_READY', {
            webkit1_patch: window.__RANKI_WEBKIT1_VLIST_PATCH__,
            katex_ready: typeof window.renderMathInElement === 'function'
        });
    }

    if (typeof document !== 'undefined') {
        if (document.readyState === 'complete' || document.readyState === 'interactive') {
            initDom();
        } else {
            window.addEventListener('DOMContentLoaded', initDom, false);
        }
    }
})();
