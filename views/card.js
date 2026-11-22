/* RAnki Route B — Resident Card Canvas Engine (views/card.js)
 * Pure MathJax 2.7.9 + Strict ES5 for Kindle PW3 WebKitGTK 1.0 (JavaScriptCore 2012)
 * Zero ES6+, zero post-2012 DOM APIs, zero Emoji, zero localStorage.
 */
(function () {
    // 1. Engine State
    var tokenCounter = 1;
    var activeGroup = 'A';       // 'A' or 'B'
    var isFlipped = false;
    var activeCardId = 0;
    var prefetchedCardId = 0;
    var activeCardCss = '';
    var prefetchedCardCss = '';

    var fontSizePx = 42;
    var uiLang = 'en';
    var mathJaxLoaded = false;
    var mathJaxLoading = false;
    var mathJaxHooked = false;
    var mathJaxWaiters = [];

    var renderGen = 1;
    var activeFrontRaw = '';
    var activeBackRaw = '';
    var prefetchedFrontRaw = '';
    var prefetchedBackRaw = '';

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
            if (code === 0x2705 || code === 0x2714) { out.push(uiLang === 'zh' ? '[\u5bf9]' : '[OK]'); continue; }
            if (code === 0x274C || code === 0x2716) { out.push(uiLang === 'zh' ? '[\u9519]' : '[X]'); continue; }
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
        var lblAudio = uiLang === 'zh' ? '[\u4e0d\u652f\u6301\u97f3\u9891]' : '[Audio unsupported]';
        var lblVideo = uiLang === 'zh' ? '[\u4e0d\u652f\u6301\u89c6\u9891/\u5d4c\u5165\u5185\u5bb9]' : '[Video/Embed unsupported]';
        var lblImage = uiLang === 'zh' ? '[\u4e0d\u652f\u6301\u56fe\u7247]' : '[Image unsupported]';
        clean = clean.replace(/\[sound:[^\]]+\]/gi, '<span class="eink-unsupported">' + lblAudio + '</span>');
        clean = clean.replace(/<audio\b[^>]*>[\s\S]*?<\/audio>/gi, '<span class="eink-unsupported">' + lblAudio + '</span>');
        clean = clean.replace(/<audio\b[^>]*\/?>/gi, '<span class="eink-unsupported">' + lblAudio + '</span>');
        clean = clean.replace(/<video\b[^>]*>[\s\S]*?<\/video>/gi, '<span class="eink-unsupported">' + lblVideo + '</span>');
        clean = clean.replace(/<video\b[^>]*\/?>/gi, '<span class="eink-unsupported">' + lblVideo + '</span>');
        clean = clean.replace(/<iframe\b[^>]*>[\s\S]*?<\/iframe>/gi, '<span class="eink-unsupported">' + lblVideo + '</span>');
        clean = clean.replace(/<object\b[^>]*>[\s\S]*?<\/object>/gi, '<span class="eink-unsupported">' + lblVideo + '</span>');
        clean = clean.replace(/<embed\b[^>]*\/?>/gi, '<span class="eink-unsupported">' + lblVideo + '</span>');
        clean = clean.replace(/<img\b[^>]*\/?>/gi, '<span class="eink-unsupported">' + lblImage + '</span>');

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

    function applyUiLang(lang) {
        uiLang = (lang === 'zh') ? 'zh' : 'en';
        if (typeof document !== 'undefined') {
            var titleEl = document.getElementById('finished-title');
            if (titleEl) {
                titleEl.textContent = (uiLang === 'zh') ? '\u4eca\u65e5\u590d\u4e60\u5df2\u5b8c\u6210' : 'Congratulations!';
            }
        }
    }

    var lastAppliedCss = null;

    function applyNoteCss(css) {
        var next = css || '';
        if (next === lastAppliedCss) return;
        lastAppliedCss = next;
        if (noteStyleEl) {
            noteStyleEl.textContent = next;
        }
    }

    // 6. Auto-scale wide equations via -webkit-transform & surgical e-ink math color normalization
    function isNeutralOrTransparent(val) {
        if (!val) return true;
        var s = String(val).toLowerCase().replace(/\s+/g, '');
        return !s || s === 'transparent' || s === 'none' || s === 'inherit' ||
            s === 'initial' || s === 'currentcolor' || s === 'rgba(0,0,0,0)';
    }

    function isPureBlackOrWhite(val) {
        var s = String(val).toLowerCase().replace(/\s+/g, '');
        return s === 'black' || s === '#000' || s === '#000000' || s === 'rgb(0,0,0)' ||
            s === 'white' || s === '#fff' || s === '#ffffff' || s === 'rgb(255,255,255)';
    }

    function normalizeMathColors(cardEl) {
        if (!cardEl || !cardEl.querySelectorAll) return;
        // Convert explicit colored TeX (\color{yellow}, \textcolor{blue}, \bbox[yellow, border:2px solid red])
        // to crisp monochrome while strictly preserving transparent layout struts & spacers!
        var styled = cardEl.querySelectorAll('.MathJax [style]');
        for (var i = 0; i < styled.length; i++) {
            var el = styled[i];
            if (!el.style) continue;

            if (el.style.color && !isNeutralOrTransparent(el.style.color)) {
                el.style.color = '#000000';
            }

            if (el.style.backgroundColor && !isNeutralOrTransparent(el.style.backgroundColor)) {
                var bgLow = String(el.style.backgroundColor).toLowerCase().replace(/\s+/g, '');
                if (bgLow !== 'white' && bgLow !== '#fff' && bgLow !== '#ffffff' &&
                    bgLow !== 'rgb(255,255,255)' && bgLow !== '#e6e6e6' && bgLow !== 'rgb(230,230,230)') {
                    el.style.backgroundColor = '#e6e6e6';
                }
            }

            var bProps = ['borderColor', 'borderTopColor', 'borderRightColor', 'borderBottomColor', 'borderLeftColor'];
            for (var b = 0; b < bProps.length; b++) {
                var bp = bProps[b];
                if (el.style[bp] && !isNeutralOrTransparent(el.style[bp]) && !isPureBlackOrWhite(el.style[bp])) {
                    el.style[bp] = '#000000';
                }
            }
        }
    }

    function unwrapScaleWrapper(target) {
        if (!target || !target.parentNode) return;
        var wrap = target.parentNode;
        if (wrap.className === 'eq-scale-wrap' && wrap.parentNode && wrap.parentNode.insertBefore) {
            wrap.parentNode.insertBefore(target, wrap);
            wrap.parentNode.removeChild(wrap);
        }
    }

    function getAvailableContainerWidth(el, cardEl, cardMaxW) {
        var avail = cardMaxW;
        var cur = el ? el.parentNode : null;
        while (cur && cur !== cardEl) {
            var tag = cur.tagName ? String(cur.tagName).toUpperCase() : '';
            if (tag === 'LI') {
                // Inside <ul>/<ol> (padding-left: 1.15em), reserve bullet + right breathing room
                // so WebKitGTK 1.0 NEVER wraps inline equations below the list marker!
                var liCap = cardMaxW - Math.ceil(fontSizePx * 1.55) - 8;
                if (liCap < 180) liCap = 180;
                if (liCap < avail) avail = liCap;
                break;
            }
            if (tag === 'TD' || tag === 'TH' || tag === 'CITE' || tag === 'BLOCKQUOTE') {
                var cw = cur.clientWidth || 0;
                if (cw > 120 && (cw - 24) < avail) {
                    avail = cw - 24;
                }
                break;
            }
            cur = cur.parentNode;
        }
        return avail;
    }

    function scaleEquationNode(target, maxW) {
        if (!target || !target.style) return;
        // Always unwrap & reset previous scale first so font-size changes never double-scale
        unwrapScaleWrapper(target);
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

            // Wrap in an explicit scaledW x scaledH layout box so WebKit 1.0 line layout
            // (especially inside <li>) sees the TRUE shrunk width without negative-margin line breaks!
            if (target.parentNode && target.parentNode.insertBefore &&
                typeof document !== 'undefined' && document.createElement) {
                var wrap = document.createElement('span');
                wrap.className = 'eq-scale-wrap';
                wrap.style.width = scaledW + 'px';
                if (scaledH > 0) {
                    wrap.style.height = scaledH + 'px';
                }
                target.parentNode.insertBefore(wrap, target);
                wrap.appendChild(target);
            } else {
                target.style.marginRight = '-' + (sw - scaledW) + 'px';
                if (sh > 0 && sh > scaledH) {
                    target.style.marginBottom = '-' + (sh - scaledH) + 'px';
                }
            }
        }
    }

    function fitWideEquations(cardEl) {
        if (!cardEl || !cardEl.querySelectorAll) return;

        // 0. Surgically normalize colored LaTeX without ever breaking transparent struts
        normalizeMathColors(cardEl);

        // 1. Cloze math polish: inline-block keeps underline cleanly BELOW tall denominators
        var clozes = cardEl.querySelectorAll('.cloze');
        for (var c = 0; c < clozes.length; c++) {
            var cl = clozes[c];
            var hasMath = cl.querySelectorAll && cl.querySelectorAll('.MathJax').length > 0;
            if (hasMath) {
                if (cl.className.indexOf('cloze-math') === -1) {
                    cl.className = cl.className + ' cloze-math';
                }
                var hasBox = false;
                var mjStyled = cl.querySelectorAll('.MathJax [style]');
                for (var m = 0; m < mjStyled.length; m++) {
                    if (mjStyled[m].style && mjStyled[m].style.borderWidth &&
                        mjStyled[m].style.borderStyle && mjStyled[m].style.borderStyle !== 'none') {
                        hasBox = true;
                        break;
                    }
                }
                if (hasBox && cl.className.indexOf('cloze-boxed') === -1) {
                    cl.className = cl.className + ' cloze-boxed';
                }
            }
        }

        // 2. Colored-emphasis math polish: keep dashed/dotted underline below fractions too
        var emphNodes = cardEl.querySelectorAll('font[color], .eink-red, .eink-color');
        for (var r = 0; r < emphNodes.length; r++) {
            var rd = emphNodes[r];
            var rdHasMath = rd.querySelectorAll && rd.querySelectorAll('.MathJax').length > 0;
            if (rdHasMath && rd.className.indexOf('eink-red-math') === -1 && rd.className.indexOf('eink-color-math') === -1) {
                var cls = (rd.className.indexOf('eink-color') !== -1) ? 'eink-color-math' : 'eink-red-math';
                rd.className = (rd.className ? rd.className + ' ' : '') + cls;
            }
        }

        // 3. Measure available horizontal card width (accounting for 42px left/right padding)
        var cardMaxW = (cardEl.clientWidth || 960) - 84;
        if (cardMaxW <= 120) cardMaxW = 860;

        // Scale display math inner .MathJax (keeps parent .MathJax_Display centering exact!)
        var dispNodes = cardEl.querySelectorAll('.MathJax_Display');
        for (var d = 0; d < dispNodes.length; d++) {
            var dispMaxW = getAvailableContainerWidth(dispNodes[d], cardEl, cardMaxW);
            var inner = dispNodes[d].querySelectorAll ? dispNodes[d].querySelectorAll('.MathJax') : [];
            if (inner.length > 0) {
                scaleEquationNode(inner[0], dispMaxW);
            } else {
                scaleEquationNode(dispNodes[d], dispMaxW);
            }
        }

        // Scale standalone inline .MathJax (measuring enclosing <li>/<td>/<cite> width accurately!)
        var inlineNodes = cardEl.querySelectorAll('.MathJax');
        for (var i = 0; i < inlineNodes.length; i++) {
            var el = inlineNodes[i];
            var p = el.parentNode;
            if (p && p.className &&
                (p.className.indexOf('MathJax_Display') !== -1 ||
                 (p.className === 'eq-scale-wrap' && p.parentNode && p.parentNode.className &&
                  p.parentNode.className.indexOf('MathJax_Display') !== -1))) {
                continue;
            }
            var itemMaxW = getAvailableContainerWidth(el, cardEl, cardMaxW);
            scaleEquationNode(el, itemMaxW);
        }
    }

    // 7. MathJax 2.7.9 Ready Hook (attaches to preloaded MathJax in card.html or loads dynamically)
    function flushMathJaxWaiters() {
        mathJaxLoaded = true;
        mathJaxLoading = false;
        var list = mathJaxWaiters.slice(0);
        mathJaxWaiters = [];
        for (var i = 0; i < list.length; i++) {
            list[i]();
        }
    }

    function ensureMathJaxLoaded(cb) {
        if (window.MathJax && window.MathJax.Hub) {
            if (mathJaxLoaded || window.MathJax.isReady) {
                mathJaxLoaded = true;
                cb();
                return;
            }
            mathJaxWaiters.push(cb);
            if (!mathJaxHooked) {
                mathJaxHooked = true;
                window.MathJax.Hub.Register.StartupHook('End', flushMathJaxWaiters);
            }
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
                mhchem: { legacy: false },
                extensions: [
                    'AMSmath.js', 'AMSsymbols.js', 'mhchem.js', 'extpfeil.js',
                    'color.js', 'bbox.js', 'cancel.js', 'enclose.js',
                    'newcommand.js', 'noErrors.js', 'noUndefined.js'
                ]
            },
            'HTML-CSS': {
                scale: 105,
                availableFonts: ['TeX'],
                preferredFont: 'TeX',
                webFont: 'TeX',
                imageFont: null,
                matchFontHeight: false,
                linebreaks: { automatic: false }
            },
            AuthorInit: function () {
                mathJaxHooked = true;
                window.MathJax.Hub.Register.StartupHook('End', flushMathJaxWaiters);
            }
        };

        var script = document.createElement('script');
        script.type = 'text/javascript';
        script.src = 'mathjax/MathJax.js?config=TeX-AMS_HTML';
        document.getElementsByTagName('head')[0].appendChild(script);
    }

    // 8. Core Typesetting Function (Pure MathJax 2.7.9 with atomic single-frame e-ink reveal)
    function renderIntoElement(cardEl, rawHtml, expectedGen, hideUntilReady, onDone) {
        var t0 = nowMs();
        if (hideUntilReady && cardEl.style) {
            cardEl.style.visibility = 'hidden';
        }
        cardEl.innerHTML = preprocessCardHtml(rawHtml);

        ensureMathJaxLoaded(function () {
            if (expectedGen && expectedGen !== renderGen) return;
            window.MathJax.Hub.Queue(['Typeset', window.MathJax.Hub, cardEl], function () {
                if (expectedGen && expectedGen !== renderGen) return;
                fitWideEquations(cardEl);
                if (cardEl.style) {
                    cardEl.style.visibility = 'visible';
                }
                if (onDone) onDone(nowMs() - t0);
            });
        });
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

    function schedulePrefetch(nextCard, gen) {
        cancelBackgroundWork();
        var inactiveGroup = (activeGroup === 'A') ? 'B' : 'A';
        var bg = getGroupSlots(inactiveGroup);

        if (!nextCard) {
            prefetchedCardId = 0;
            prefetchedCardCss = '';
            prefetchedFrontRaw = '';
            prefetchedBackRaw = '';
            if (bg.frontCard) bg.frontCard.innerHTML = '';
            if (bg.backCard) bg.backCard.innerHTML = '';
            return;
        }

        prefetchedCardId = 0; // Only mark ready after both sides finish rendering
        prefetchedCardCss = nextCard.css || '';
        prefetchedFrontRaw = nextCard.question_html || '';
        prefetchedBackRaw = nextCard.answer_html || '';
        var expectedGen = gen || renderGen;

        pendingPrefetchTimer = setTimeout(function () {
            pendingPrefetchTimer = null;
            if (expectedGen !== renderGen) return;
            renderIntoElement(bg.frontCard, prefetchedFrontRaw, expectedGen, false, function () {
                if (expectedGen !== renderGen) return;
                renderIntoElement(bg.backCard, prefetchedBackRaw, expectedGen, false, function () {
                    if (expectedGen !== renderGen) return;
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
        _getAvailableContainerWidth: getAvailableContainerWidth,
        _normalizeMathColors: normalizeMathColors,

        setConfig: function (cfg) {
            if (!cfg) return;
            if (typeof cfg.font_size_px === 'number') {
                applyFontSize(cfg.font_size_px);
            }
            if (typeof cfg.lang === 'string') {
                applyUiLang(cfg.lang);
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
            renderGen++;
            var myGen = renderGen;
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
            activeFrontRaw = cur.question_html || '';
            activeBackRaw = cur.answer_html || '';
            prefetchedCardId = 0;
            applyNoteCss(activeCardCss);

            var g = getGroupSlots('A');
            resetPageIndex(g.frontView);
            g.backView.scrollTop = 0;
            setVisibleSlot(g.frontView);

            // Atomic e-ink reveal: keep front hidden during MathJax typesetting so raw TeX never ghosts!
            renderIntoElement(g.frontCard, activeFrontRaw, myGen, true, function (elapsedMs) {
                if (myGen !== renderGen) return;
                var st = emitPageState();
                notifyHost('RENDER_DONE', {
                    card_id: activeCardId,
                    mode: 'immediate',
                    engine: 'mathjax',
                    ms: elapsedMs,
                    page: st.page,
                    total: st.total
                });

                // Render Back cleanly right after Front paints, then prefetch Next
                var backHtml = activeBackRaw;
                var backCid = activeCardId;
                pendingBackRender = function () {
                    if (myGen !== renderGen || activeCardId !== backCid) return;
                    renderIntoElement(g.backCard, backHtml, myGen, isFlipped, function () {
                        if (myGen !== renderGen || activeCardId !== backCid) return;
                        if (isFlipped) {
                            emitPageState();
                        }
                        schedulePrefetch(next, myGen);
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
            isFlipped = true;
            flushPendingBack();
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

            renderGen++;
            var myGen = renderGen;
            cancelBackgroundWork();
            pendingBackRender = null;

            activeGroup = (activeGroup === 'A') ? 'B' : 'A';
            isFlipped = false;
            activeCardId = expectedId;
            activeCardCss = prefetchedCardCss;
            activeFrontRaw = prefetchedFrontRaw;
            activeBackRaw = prefetchedBackRaw;
            applyNoteCss(activeCardCss);

            var g = getGroupSlots(activeGroup);
            if (g.frontCard && g.frontCard.style) g.frontCard.style.visibility = 'visible';
            if (g.backCard && g.backCard.style) g.backCard.style.visibility = 'visible';
            resetPageIndex(g.frontView);
            g.backView.scrollTop = 0;
            setVisibleSlot(g.frontView);

            var st = emitPageState();
            notifyHost('RENDER_DONE', {
                card_id: activeCardId,
                mode: 'prefetch_hit',
                engine: 'mathjax',
                ms: nowMs() - t0,
                page: st.page,
                total: st.total
            });

            schedulePrefetch(payload.prefetch_next || null, myGen);
            return true;
        },

        pageScroll: pageScroll,

        showFinished: function (subtitle) {
            renderGen++;
            cancelBackgroundWork();
            pendingBackRender = null;
            activeCardId = 0;
            prefetchedCardId = 0;
            activeFrontRaw = '';
            activeBackRaw = '';
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
                uiLang: uiLang,
                mathEngine: 'mathjax'
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
            engine: 'mathjax',
            mathjax_present: !!(window.MathJax && window.MathJax.Hub)
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
