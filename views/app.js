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
            listEl.innerHTML = '<div class="empty-msg">没有可复习的牌组</div>';
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

    // Queue Lock: Flip card when user taps 'Show Answer'
    function flipCard() {
        if (isFlipped) {
            return;
        }

        // If MathJax typesetting is still running in #card-buffer, queue the flip
        if (!isTypesettingDone) {
            console.log("[RAnki-SPA] Typesetting not complete. Queue Lock engaged.");
            pendingFlip = true;
            return;
        }

        executeDomFlip();
    }

    function executeDomFlip() {
        var active = document.getElementById('card-active');
        var buffer = document.getElementById('card-buffer');
        if (!active || !buffer) {
            return;
        }

        // In-place physical DOM swap (<= 5ms)
        buffer.style.position = 'static';
        buffer.style.left = '0';
        buffer.style.visibility = 'visible';

        active.style.position = 'absolute';
        active.style.left = '-9999px';
        active.style.visibility = 'hidden';

        active.id = 'card-buffer';
        buffer.id = 'card-active';

        isFlipped = true;
        pendingFlip = false;

        document.getElementById('bar-unflipped').style.display = 'none';
        document.getElementById('bar-flipped').style.display = 'block';

        sendHost('CARD_FLIPPED', {});
    }

    function renderCard(data) {
        currentView = 'reviewer';
        document.getElementById('deck-view').style.display = 'none';
        document.getElementById('review-view').style.display = 'block';

        isFlipped = false;
        pendingFlip = false;
        isTypesettingDone = false;

        var active = document.getElementById('card-active');
        var buffer = document.getElementById('card-buffer');

        // Front question goes to active viewport
        active.innerHTML = data.question_html || '';

        // Back answer goes to offscreen buffer (left: -9999px)
        buffer.innerHTML = data.answer_html || '';

        document.getElementById('bar-unflipped').style.display = 'block';
        document.getElementById('bar-flipped').style.display = 'none';

        // Typeset buffer with MathJax if available
        if (window.MathJax && window.MathJax.Hub) {
            MathJax.Hub.Queue(["Typeset", MathJax.Hub, buffer], function () {
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

    // Attach click events to DOM elements
    window.onload = function () {
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
