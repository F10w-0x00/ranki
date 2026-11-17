/* RAnki Unified Web-SPA Frontend Engine (views/app.js)
 * Strict ES5 syntax for WebKit 1.0 (JavaScriptCore 2012)
 * No let, const, =>, class, Promise, async/await or localStorage
 */

(function () {
    var tokenCounter = 1;
    var currentView = 'deck-picker'; // 'deck-picker' or 'reviewer'
    var isFlipped = false;
    var isTypesettingDone = true;
    var pendingFlip = false;

    var activeEl = null;
    var bufferEl = null;
    var styleEl = null;
    var cardAreaEl = null;

    // Send up-channel command to Rust Host
    function sendHost(action, payload) {
        var token = tokenCounter++;
        var payloadStr = typeof payload === 'string' ? payload : JSON.stringify(payload || {});
        document.title = "RA:" + action + ":" + token + ":" + payloadStr;
    }

    // Down-channel dispatcher called by Rust Host: window.RAnki.dispatch(event, payload)
    window.RAnki = {
        dispatch: function (event, payload) {
            console.log("[RAnki-SPA] Received event: " + event);
            if (event === 'HOST_ACK') {
                console.log("[RAnki-SPA] Host handshake acknowledged.");
            } else if (event === 'LOAD_DECK_LIST') {
                renderDeckList(payload);
            } else if (event === 'SHOW_CARD') {
                renderCard(payload);
            }
        }
    };

    function renderDeckList(data) {
        currentView = 'deck-picker';
        document.getElementById('deck-view').style.display = 'block';
        document.getElementById('review-view').style.display = 'none';

        var listEl = document.getElementById('deck-list');
        listEl.innerHTML = '';

        var decks = (data && data.decks) || [];
        if (decks.length === 0) {
            listEl.innerHTML = '<div class="empty-msg" style="padding: 24px; font-size: 30px;">没有可复习的牌组</div>';
            return;
        }

        for (var i = 0; i < decks.length; i++) {
            var deck = decks[i];
            var item = document.createElement('div');
            item.className = 'deck-item';
            item.setAttribute('data-id', deck.id);
            item.innerHTML = '<span class="deck-name">' + deck.name + '</span>' +
                '<span class="deck-counts">' +
                '<span class="count-due">' + (deck.due_count || 0) + '</span>' +
                '</span>';

            item.onclick = (function (dId) {
                return function () {
                    sendHost('SELECT_DECK', { deck_id: dId });
                };
            })(deck.id);

            listEl.appendChild(item);
        }
    }

    // Preprocess card HTML for chemistry and math syntax
    function preprocessCardHtml(html) {
        if (!html) return '';

        // 1. Temporarily protect existing math blocks: $...$, $$...$$, \(...\), \[...\]
        var mathBlocks = [];
        var protectedHtml = html.replace(/(\$\$[\s\S]*?\$\$|\\\[[\s\S]*?\\\]|\$[\s\S]*?\$|\\\([\s\S]*?\\\))/g, function (match) {
            mathBlocks.push(match);
            return '___MATH_BLOCK_' + (mathBlocks.length - 1) + '___';
        });

        // 2. Wrap standalone \ce{...} that were not wrapped in math delimiters
        protectedHtml = protectedHtml.replace(/(\\ce\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\})/g, function (match) {
            return '\\(' + match + '\\)';
        });

        // 3. Restore protected math blocks
        var restoredHtml = protectedHtml.replace(/___MATH_BLOCK_(\d+)___/g, function (match, idx) {
            return mathBlocks[parseInt(idx, 10)];
        });

        return restoredHtml;
    }

    // Queue Lock: Flip card when user taps 'Show Answer'
    function flipCard() {
        if (isFlipped) {
            return;
        }

        // If MathJax typesetting is still running in offscreen buffer, engage Queue Lock
        if (!isTypesettingDone) {
            console.log("[RAnki-SPA] Typesetting not complete. Queue Lock engaged.");
            pendingFlip = true;
            return;
        }

        executeDomFlip();
    }

    function executeDomFlip() {
        if (!activeEl || !bufferEl) {
            return;
        }

        // In-place physical DOM class & pointer swap (<= 5ms)
        bufferEl.className = "card-box";
        activeEl.className = "card-box offscreen";

        var temp = activeEl;
        activeEl = bufferEl;
        bufferEl = temp;

        isFlipped = true;
        pendingFlip = false;

        document.getElementById('bar-unflipped').style.display = 'none';
        document.getElementById('bar-flipped').style.display = 'block';

        // Reset scroll position on answer reveal
        if (cardAreaEl) {
            cardAreaEl.scrollTop = 0;
        }

        sendHost('CARD_FLIPPED', {});
    }

    function renderCard(data) {
        currentView = 'reviewer';
        document.getElementById('deck-view').style.display = 'none';
        document.getElementById('review-view').style.display = 'block';

        isFlipped = false;
        pendingFlip = false;
        isTypesettingDone = false;

        // Reset scroll to top for new card
        if (cardAreaEl) {
            cardAreaEl.scrollTop = 0;
        }

        // Inject card template CSS
        if (styleEl && data.css) {
            styleEl.textContent = data.css;
        }

        // Update intervals on rating buttons
        if (data.intervals && data.intervals.length >= 4) {
            var againEl = document.getElementById('interval-again');
            if (againEl) againEl.textContent = data.intervals[0];

            var hardEl = document.getElementById('interval-hard');
            if (hardEl) hardEl.textContent = data.intervals[1];

            var goodEl = document.getElementById('interval-good');
            if (goodEl) goodEl.textContent = data.intervals[2];

            var easyEl = document.getElementById('interval-easy');
            if (easyEl) easyEl.textContent = data.intervals[3];
        }

        // Preprocess and inject Front question
        activeEl.innerHTML = preprocessCardHtml(data.question_html || '');

        // Preprocess and inject Back answer into offscreen buffer slot
        bufferEl.innerHTML = preprocessCardHtml(data.answer_html || '');

        document.getElementById('bar-unflipped').style.display = 'block';
        document.getElementById('bar-flipped').style.display = 'none';

        // Dual-buffer MathJax typesetting
        if (window.MathJax && window.MathJax.Hub && window.MathJax.Hub.Queue) {
            // 1. Typeset active front
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, activeEl]);

            // 2. Typeset offscreen back buffer and signal readiness
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, bufferEl], function () {
                isTypesettingDone = true;
                if (pendingFlip) {
                    executeDomFlip();
                }
            });
        } else {
            isTypesettingDone = true;
        }
    }

    function rateCard(rating) {
        sendHost('RATE_CARD', { rating: rating });
    }

    function exitApp() {
        sendHost('exit', {});
    }

    function backToDecks() {
        sendHost('EXIT_DECK', {});
    }

    // Attach click events & touch drag scrolling
    window.onload = function () {
        activeEl = document.getElementById('card-active');
        bufferEl = document.getElementById('card-buffer');
        styleEl = document.getElementById('card-dynamic-style');
        cardAreaEl = document.querySelector('.card-area');

        // Touch & Mouse Drag Scrolling for Kindle WebKit 1.0
        if (cardAreaEl) {
            var touchStartY = 0;
            var touchStartScrollTop = 0;
            var isTouching = false;

            cardAreaEl.addEventListener('touchstart', function (e) {
                if (e.touches && e.touches.length > 0) {
                    isTouching = true;
                    touchStartY = e.touches[0].pageY;
                    touchStartScrollTop = cardAreaEl.scrollTop;
                }
            }, false);

            cardAreaEl.addEventListener('touchmove', function (e) {
                if (!isTouching || !e.touches || e.touches.length === 0) return;
                var deltaY = e.touches[0].pageY - touchStartY;
                cardAreaEl.scrollTop = touchStartScrollTop - deltaY;
            }, false);

            cardAreaEl.addEventListener('touchend', function () {
                isTouching = false;
            }, false);

            // Mouse drag fallback for X11 pointer events
            var isMouseDown = false;
            var mouseStartY = 0;
            var mouseStartScrollTop = 0;

            cardAreaEl.addEventListener('mousedown', function (e) {
                isMouseDown = true;
                mouseStartY = e.pageY;
                mouseStartScrollTop = cardAreaEl.scrollTop;
            }, false);

            window.addEventListener('mousemove', function (e) {
                if (!isMouseDown) return;
                var deltaY = e.pageY - mouseStartY;
                cardAreaEl.scrollTop = mouseStartScrollTop - deltaY;
            }, false);

            window.addEventListener('mouseup', function () {
                isMouseDown = false;
            }, false);
        }

        var btnShowAnswer = document.getElementById('btn-show-answer');
        if (btnShowAnswer) {
            btnShowAnswer.onclick = flipCard;
        }

        var btnExit = document.getElementById('btn-exit');
        if (btnExit) {
            btnExit.onclick = exitApp;
        }

        var btnBack = document.getElementById('btn-back');
        if (btnBack) {
            btnBack.onclick = backToDecks;
        }

        var ratings = ['again', 'hard', 'good', 'easy'];
        for (var i = 0; i < ratings.length; i++) {
            (function (r) {
                var el = document.getElementById('btn-' + r);
                if (el) {
                    el.onclick = function () {
                        rateCard(r);
                    };
                }
            })(ratings[i]);
        }

        // Notify Host that Web-SPA is loaded
        sendHost('ready', {});
    };
})();
