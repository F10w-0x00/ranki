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

    var fontSizePx = 42;
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

    // 4. Anti-Tofu Sanitizer + Unsupported Media Fallback + Unified Monochrome Color Normalizer
    function sanitizeUnicodeForKindle(str) {
        if (!str) return '';
        var out = [];
        for (var i = 0; i < str.length; i++) {
            var code = str.charCodeAt(i);

            // Strip invisible bidi / zero-width / BOM / C0 control chars (except \t, \n, \r)
            if (code === 0x2068 || code === 0x2069 || code === 0xFEFF ||
                (code >= 0x200B && code <= 0x200F) ||
                (code < 0x20 && code !== 0x09 && code !== 0x0A && code !== 0x0D)) {
                continue;
            }

            // Handle astral-plane Emoji surrogate pairs (U+1F000..U+1FAFF -> tofu on Kindle OS 5.14)
            if (code >= 0xD83C && code <= 0xDBFF && i + 1 < str.length) {
                var low = str.charCodeAt(i + 1);
                if (low >= 0xDC00 && low <= 0xDFFF) {
                    i++; // consume low surrogate
                    // Skip trailing variation selector U+FE0F if present
                    if (i + 1 < str.length && str.charCodeAt(i + 1) === 0xFE0F) i++;
                    continue;
                }
            }

            // Replace common BMP Emoji / Dingbat glyphs missing on Kindle OS 5.14
            if (code === 0x2705 || code === 0x2714) { out.push('[\u5bf9]'); continue; }
            if (code === 0x274C || code === 0x2716) { out.push('[\u9519]'); continue; }
            if (code === 0x26A0) {
                if (i + 1 < str.length && str.charCodeAt(i + 1) === 0xFE0F) i++;
                out.push('[!]');
                continue;
            }
            if (code === 0xFE0F) continue;
            if (code >= 0x2600 && code <= 0x27BF) {
                // Keep basic geometry/check/star/bullet symbols that exist in standard fonts? No, strip tofu risk
                continue;
            }

            out.push(str.charAt(i));
        }
        return out.join('');
    }

    function classifyColorValue(raw) {
        if (!raw) return 'neutral';
        var c = String(raw).toLowerCase().replace(/\s+/g, '');
        if (!c || c === 'inherit' || c === 'initial' || c === 'currentcolor' || c === 'transparent') {
            return 'neutral';
        }
        if (c === 'black' || c === 'gray' || c === 'grey' || c === 'darkgray' || c === 'dimgray' ||
            c === '#000' || c === '#000000' || c === '#111' || c === '#111111' ||
            c === '#222' || c === '#222222' || c === '#333' || c === '#333333' ||
            c === '#444' || c === '#444444' || c === '#555' || c === '#555555' ||
            c === '#666' || c === '#666666' || c === '#777' || c === '#777777' ||
            c === '#888' || c === '#888888') {
            return 'neutral';
        }
        // Check hex RGB to distinguish true neutral gray vs warning red/orange vs other hues
        var hexMatch = c.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/);
        if (hexMatch) {
            var h = hexMatch[1];
            var r, g, b;
            if (h.length === 3) {
                r = parseInt(h.charAt(0) + h.charAt(0), 16);
                g = parseInt(h.charAt(1) + h.charAt(1), 16);
                b = parseInt(h.charAt(2) + h.charAt(2), 16);
            } else {
                r = parseInt(h.substring(0, 2), 16);
                g = parseInt(h.substring(2, 4), 16);
                b = parseInt(h.substring(4, 6), 16);
            }
            if (Math.abs(r - g) <= 18 && Math.abs(g - b) <= 18 && Math.abs(r - b) <= 18) {
                return 'neutral';
            }
            if (r >= 170 && r > g + 55 && r > b + 55) {
                return 'red';
            }
            return 'color';
        }
        var rgbMatch = c.match(/^rgba?\((\d+),(\d+),(\d+)/);
        if (rgbMatch) {
            var rr = parseInt(rgbMatch[1], 10);
            var gg = parseInt(rgbMatch[2], 10);
            var bb = parseInt(rgbMatch[3], 10);
            if (Math.abs(rr - gg) <= 18 && Math.abs(gg - bb) <= 18 && Math.abs(rr - bb) <= 18) {
                return 'neutral';
            }
            if (rr >= 170 && rr > gg + 55 && rr > bb + 55) {
                return 'red';
            }
            return 'color';
        }
        if (c === 'red' || c === 'crimson' || c === 'firebrick' || c === 'darkred' ||
            c === 'tomato' || c === 'orangered' || c === 'orange' || c === 'coral' ||
            c === 'magenta' || c === 'deeppink') {
            return 'red';
        }
        return 'color';
    }

    function appendClassToOpenTag(openTag, cls) {
        if (openTag.indexOf(cls) !== -1) return openTag;
        if (/class\s*=\s*["']/i.test(openTag)) {
            return openTag.replace(/class\s*=\s*(["'])/i, 'class=$1' + cls + ' ');
        }
        return openTag + ' class="' + cls + '"';
    }

    function preprocessCardHtml(html) {
        if (!html) return '';

        // 1. Strip dangerous/unused executable tags
        var clean = String(html).replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');

        // 2. Replace unsupported media with clean monochrome notice badges (zero tofu / broken icons)
        clean = clean.replace(/\[sound:[^\]]+\]/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u97f3\u9891]</span>');
        clean = clean.replace(/<audio\b[^>]*>[\s\S]*?<\/audio>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u97f3\u9891]</span>');
        clean = clean.replace(/<audio\b[^>]*\/?>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u97f3\u9891]</span>');
        clean = clean.replace(/<video\b[^>]*>[\s\S]*?<\/video>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]</span>');
        clean = clean.replace(/<video\b[^>]*\/?>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]</span>');
        clean = clean.replace(/<iframe\b[^>]*>[\s\S]*?<\/iframe>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]</span>');
        clean = clean.replace(/<object\b[^>]*>[\s\S]*?<\/object>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]</span>');
        clean = clean.replace(/<embed\b[^>]*\/?>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]</span>');
        clean = clean.replace(/<img\b[^>]*\/?>/gi, '<span class="eink-unsupported">[\u4e0d\u652f\u6301\u56fe\u7247]</span>');

        // 3. Anti-tofu sanitization (strip invisible control chars & Kindle-unsupported Emoji)
        clean = sanitizeUnicodeForKindle(clean);

        // 4. Protect existing math delimiters before scanning for bare \ce{...}
        var mathBlocks = [];
        var protectedHtml = clean.replace(/(\$\$[\s\S]*?\$\$|\\\[[\s\S]*?\\\]|\$[\s\S]*?\$|\\\([\s\S]*?\\\))/g, function (match) {
            mathBlocks.push(match);
            return '___RANKI_MATH_' + (mathBlocks.length - 1) + '___';
        });

        // 5. Wrap standalone \ce{...} in inline math delimiters
        protectedHtml = protectedHtml.replace(/(\\ce\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\})/g, function (match) {
            return '\\(' + match + '\\)';
        });

        // 6. Unified E-ink Color Normalization for <font color="..."> and inline style="color:...; background-color:..."
        protectedHtml = protectedHtml.replace(/<font\b([^>]*)>/gi, function (all, attrs) {
            var m = attrs.match(/\bcolor\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/i);
            if (!m) return all;
            var val = m[1] || m[2] || m[3] || '';
            var kind = classifyColorValue(val);
            if (kind === 'red') return appendClassToOpenTag('<font' + attrs, 'eink-red') + '>';
            if (kind === 'color') return appendClassToOpenTag('<font' + attrs, 'eink-color') + '>';
            return all;
        });

        protectedHtml = protectedHtml.replace(/<([a-z][a-z0-9]*)\b([^>]*\bstyle\s*=\s*(?:"[^"]*"|'[^']*')[^>]*)>/gi, function (all, tag, attrs) {
            var styleMatch = attrs.match(/\bstyle\s*=\s*(?:"([^"]*)"|'([^']*)')/i);
            if (!styleMatch) return all;
            var styleText = styleMatch[1] || styleMatch[2] || '';
            var openTag = '<' + tag + attrs;
            var changed = false;

            // Match foreground color: (not background-color:)
            var colorMatch = styleText.match(/(?:^|;)\s*color\s*:\s*([^;]+)/i);
            if (colorMatch) {
                var kind = classifyColorValue(colorMatch[1]);
                if (kind === 'red') { openTag = appendClassToOpenTag(openTag, 'eink-red'); changed = true; }
                else if (kind === 'color') { openTag = appendClassToOpenTag(openTag, 'eink-color'); changed = true; }
            }

            // Match non-white/non-transparent highlight background-color:
            var bgMatch = styleText.match(/(?:^|;)\s*background(?:-color)?\s*:\s*([^;]+)/i);
            if (bgMatch) {
                var bgVal = bgMatch[1].toLowerCase().replace(/\s+/g, '');
                if (bgVal && bgVal !== 'white' && bgVal !== '#fff' && bgVal !== '#ffffff' &&
                    bgVal !== 'transparent' && bgVal !== 'none' && bgVal !== 'inherit') {
                    openTag = appendClassToOpenTag(openTag, 'eink-mark');
                    changed = true;
                }
            }
            return changed ? (openTag + '>') : all;
        });

        // 7. Restore protected math blocks
        var restored = protectedHtml.replace(/___RANKI_MATH_(\d+)___/g, function (match, idx) {
            return mathBlocks[parseInt(idx, 10)];
        });

        return restored;
    }

    // 5. Font Size & Dynamic Card Style Management (28..54px, default 42px @ 300 PPI)
    function applyFontSize(px) {
        var clamped = parseInt(px, 10);
        if (isNaN(clamped)) clamped = 42;
        if (clamped < 28) clamped = 28;
        if (clamped > 54) clamped = 54;
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

    // 6. Auto-scale wide equations via -webkit-transform (centered layout collapse, zero clipping)
    function scaleEquationNode(target, maxW) {
        if (!target || !target.style) return;
        // Always reset previous scale first so font-size changes never double-scale
        target.style.webkitTransform = '';
        target.style.transform = '';
        target.style.webkitTransformOrigin = '';
        target.style.transformOrigin = '';
        target.style.marginRight = '';
        target.style.marginBottom = '';

        var sw = target.scrollWidth || target.offsetWidth || 0;
        var sh = target.offsetHeight || target.scrollHeight || 0;
        if (sw > maxW && sw > 0) {
            var ratio = Math.floor((maxW / sw) * 1000) / 1000;
            if (ratio < 0.38) ratio = 0.38;
            var scaledW = Math.ceil(sw * ratio);
            var scaledH = Math.ceil(sh * ratio);
            target.style.display = 'inline-block';
            target.style.webkitTransformOrigin = '0 0';
            target.style.transformOrigin = '0 0';
            target.style.webkitTransform = 'scale(' + ratio + ')';
            target.style.transform = 'scale(' + ratio + ')';
            target.style.marginRight = '-' + (sw - scaledW) + 'px';
            if (sh > 0 && sh > scaledH) {
                target.style.marginBottom = '-' + (sh - scaledH) + 'px';
            }
        }
    }

    function fitWideEquations(cardEl) {
        if (!cardEl || !cardEl.querySelectorAll) return;

        // 1. Cloze math polish: inline-block keeps underline cleanly BELOW tall denominators
        var clozes = cardEl.querySelectorAll('.cloze');
        for (var c = 0; c < clozes.length; c++) {
            var cl = clozes[c];
            var hasMath = cl.querySelectorAll && cl.querySelectorAll('.katex, .MathJax').length > 0;
            if (hasMath) {
                if (cl.className.indexOf('cloze-math') === -1) {
                    cl.className = cl.className + ' cloze-math';
                }
                var hasBox = cl.querySelectorAll('.fbox, .fcolorbox').length > 0;
                if (hasBox && cl.className.indexOf('cloze-boxed') === -1) {
                    cl.className = cl.className + ' cloze-boxed';
                }
            }
        }

        // 2. Colored-emphasis math polish: keep dashed/dotted underline below fractions too
        var emphNodes = cardEl.querySelectorAll('font[color], .eink-red, .eink-color');
        for (var r = 0; r < emphNodes.length; r++) {
            var rd = emphNodes[r];
            var rdHasMath = rd.querySelectorAll && rd.querySelectorAll('.katex, .MathJax').length > 0;
            if (rdHasMath && rd.className.indexOf('eink-red-math') === -1 && rd.className.indexOf('eink-color-math') === -1) {
                var cls = (rd.className.indexOf('eink-color') !== -1) ? 'eink-color-math' : 'eink-red-math';
                rd.className = (rd.className ? rd.className + ' ' : '') + cls;
            }
        }

        // 3. Measure available horizontal card width (accounting for 42px left/right padding)
        var maxW = (cardEl.clientWidth || 960) - 84;
        if (maxW <= 120) maxW = 860;

        // Scale display math inner .katex (keeps parent .katex-display centering exact!)
        var dispNodes = cardEl.querySelectorAll('.katex-display');
        for (var d = 0; d < dispNodes.length; d++) {
            var inner = dispNodes[d].querySelectorAll ? dispNodes[d].querySelectorAll('.katex') : [];
            if (inner.length > 0) {
                scaleEquationNode(inner[0], maxW);
            } else {
                scaleEquationNode(dispNodes[d], maxW);
            }
        }

        // Scale standalone inline .katex (skipping those inside .katex-display)
        var inlineNodes = cardEl.querySelectorAll('.katex, .MathJax_Display');
        for (var i = 0; i < inlineNodes.length; i++) {
            var el = inlineNodes[i];
            if (el.parentNode && el.parentNode.className &&
                el.parentNode.className.indexOf('katex-display') !== -1) {
                continue;
            }
            scaleEquationNode(el, maxW);
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
        _scaleEquationNode: scaleEquationNode,

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

        // Only mount Kindle file:// system font path when running from file:// (avoids browser HTTP security error)
        if (typeof window !== 'undefined' && window.location && window.location.protocol === 'file:') {
            var kFontStyle = document.createElement('style');
            kFontStyle.textContent =
                "@font-face { font-family: 'SonyReader Ming'; src: local('SonyReader Ming'), url('file:///mnt/us/fonts/SonyReader%20Ming.ttf') format('truetype'); font-weight: normal; font-style: normal; }";
            var head = document.getElementsByTagName('head')[0];
            if (head) head.appendChild(kFontStyle);
        }

        notifyHost('CARD_READY', {
            webkit1_patch: window.__RANKI_WEBKIT1_VLIST_PATCH__,
            katex_ready: typeof window.renderMathInElement === 'function'
        });
    }

    // Optional postMessage bridge so local file:// preview_cards.html can drive CardEngine
    if (typeof window !== 'undefined' && window.addEventListener) {
        window.addEventListener('message', function (ev) {
            var msg = ev && ev.data;
            if (!msg || msg.target !== 'ranki-card' || !msg.method) return;
            if (typeof CardEngine[msg.method] === 'function') {
                CardEngine[msg.method](msg.arg);
            }
        }, false);
    }

    if (typeof document !== 'undefined') {
        if (document.readyState === 'complete' || document.readyState === 'interactive') {
            initDom();
        } else {
            window.addEventListener('DOMContentLoaded', initDom, false);
        }
    }
})();
