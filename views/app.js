/* RAnki Unified Web-SPA Frontend Engine (views/app.js)
 * Strict ES5 syntax for WebKit 1.0 (JavaScriptCore 2012)
 * Quad-Slot Ping-Pong Pre-Rendering Pipeline & Anki-Authentic Deck Browser
 */

(function () {
    var tokenCounter = 1;
    var currentView = 'deck-picker'; // 'deck-picker' or 'reviewer'

    // Quad-slot DOM elements
    var slotAFront = null;
    var slotABack = null;
    var slotBFront = null;
    var slotBBack = null;

    var styleEl = null;
    var cardAreaEl = null;
    var panelFinished = null;

    // Pipeline State Machine
    var currentGroup = 'A';       // 'A' or 'B'
    var isFlipped = false;
    var isTypesettingDone = true;  // Queue lock for current card back
    var isNextReady = true;        // Queue lock for next card
    var hasNextCard = false;
    var pendingFlip = false;
    var pendingNext = false;
    var pendingRating = '';

    // Active card metadata
    var currentCardData = null;
    var nextCardData = null;

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
            } else if (event === 'SHOW_CARD_PIPELINE') {
                renderCardPipeline(payload);
            } else if (event === 'PREPARE_NEXT_CARD') {
                handlePrepareNextCard(payload);
            } else if (event === 'SHOW_CARD') {
                // Fallback for single card payload
                renderCardFallback(payload);
            }
        }
    };

    function escapeHtml(str) {
        if (!str) return '';
        return String(str)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
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

    // 1. Render Anki-Authentic Centered Deck Browser
    function renderDeckList(data) {
        currentView = 'deck-picker';
        document.getElementById('deck-view').style.display = 'block';
        document.getElementById('review-view').style.display = 'none';

        var titleEl = document.getElementById('deck-title');
        if (titleEl) {
            titleEl.textContent = 'RAnki 牌组大厅';
        }
        var badgesEl = document.getElementById('header-badges');
        if (badgesEl) {
            badgesEl.style.display = 'none';
        }
        var backBtn = document.getElementById('btn-back');
        if (backBtn) {
            backBtn.style.visibility = 'hidden';
        }

        var listEl = document.getElementById('deck-list');
        listEl.innerHTML = '';

        var decks = (data && data.decks) || [];
        if (decks.length === 0) {
            listEl.innerHTML = '<div class="empty-msg" style="padding: 40px; text-align: center; font-size: 28px;">没有可复习的牌组</div>';
            return;
        }

        var totalDue = 0;
        for (var i = 0; i < decks.length; i++) {
            var deck = decks[i];
            totalDue += (deck.due_count || 0);

            var item = document.createElement('div');
            item.className = 'deck-item';
            item.setAttribute('data-id', deck.id);

            var depth = deck.depth || 0;
            var indentPx = depth * 32;
            var prefix = depth > 0 ? '├─ ' : '▾ ';

            var newCount = typeof deck.new_count !== 'undefined' ? deck.new_count : '-';
            var learnCount = typeof deck.learn_count !== 'undefined' ? deck.learn_count : '-';
            var reviewCount = typeof deck.review_count !== 'undefined' ? deck.review_count : (deck.due_count || 0);

            var newClass = (newCount === 0 || newCount === '-') ? 'deck-count zero' : 'deck-count';
            var learnClass = (learnCount === 0 || learnCount === '-') ? 'deck-count zero' : 'deck-count';
            var reviewClass = (reviewCount === 0 || reviewCount === '-') ? 'deck-count zero' : 'deck-count';

            var nameHtml = '<span class="deck-name" style="padding-left: ' + indentPx + 'px;">' +
                '<span style="font-family: monospace; font-size: 24px; color: #444;">' + prefix + '</span>' +
                escapeHtml(deck.name) +
                '</span>';

            var countsHtml = '<div class="deck-counts-group">' +
                '<span class="' + newClass + '">' + newCount + '</span>' +
                '<span class="' + learnClass + '">' + learnCount + '</span>' +
                '<span class="' + reviewClass + '">' + reviewCount + '</span>' +
                '</div>';

            item.innerHTML = nameHtml + countsHtml;

            item.onclick = (function (dId) {
                return function () {
                    sendHost('SELECT_DECK', { deck_id: dId });
                };
            })(deck.id);

            listEl.appendChild(item);
        }

        var summaryEl = document.getElementById('deck-summary');
        if (summaryEl) {
            summaryEl.textContent = '今日待复习卡片总数: ' + totalDue + ' 张';
        }
    }

    // Update interval labels on rating buttons
    function updateIntervals(intervals) {
        if (!intervals || intervals.length < 4) {
            intervals = ['< 10分', '1天', '3天', '5天'];
        }
        var againEl = document.getElementById('interval-again');
        if (againEl) againEl.textContent = intervals[0];

        var hardEl = document.getElementById('interval-hard');
        if (hardEl) hardEl.textContent = intervals[1];

        var goodEl = document.getElementById('interval-good');
        if (goodEl) goodEl.textContent = intervals[2];

        var easyEl = document.getElementById('interval-easy');
        if (easyEl) easyEl.textContent = intervals[3];
    }

    // 2. Render Card Pipeline (SHOW_CARD_PIPELINE)
    function renderCardPipeline(data) {
        currentView = 'reviewer';
        document.getElementById('deck-view').style.display = 'none';
        document.getElementById('review-view').style.display = 'block';

        if (panelFinished) {
            panelFinished.style.display = 'none';
        }

        var backBtn = document.getElementById('btn-back');
        if (backBtn) {
            backBtn.style.visibility = 'visible';
        }

        var titleEl = document.getElementById('deck-title');
        if (titleEl && data.deck_name) {
            titleEl.textContent = data.deck_name;
        }

        var badgesEl = document.getElementById('header-badges');
        if (badgesEl) {
            badgesEl.style.display = 'flex';
            var total = (data.new_count || 0) + (data.learn_count || 0) + (data.review_count || 0);
            badgesEl.innerHTML = '<span class="badge-item">' + total + '</span>';
        }

        currentGroup = 'A';
        isFlipped = false;
        pendingFlip = false;
        pendingNext = false;
        isTypesettingDone = false;
        isNextReady = false;

        currentCardData = data.current;
        nextCardData = data.next;

        if (cardAreaEl) {
            cardAreaEl.scrollTop = 0;
        }

        // Inject dynamic card CSS
        if (styleEl && currentCardData && currentCardData.css) {
            styleEl.textContent = currentCardData.css;
        }

        updateIntervals(currentCardData ? currentCardData.intervals : null);

        // Group A: Active Card
        slotAFront.className = 'card-box';
        slotABack.className = 'card-box offscreen';
        slotAFront.innerHTML = preprocessCardHtml(currentCardData.question_html || '');
        slotABack.innerHTML = preprocessCardHtml(currentCardData.answer_html || '');

        // Group B: Pre-render Next Card (if available)
        slotBFront.className = 'card-box offscreen';
        slotBBack.className = 'card-box offscreen';

        if (nextCardData) {
            hasNextCard = true;
            slotBFront.innerHTML = preprocessCardHtml(nextCardData.question_html || '');
            slotBBack.innerHTML = preprocessCardHtml(nextCardData.answer_html || '');
        } else {
            hasNextCard = false;
            slotBFront.innerHTML = '';
            slotBBack.innerHTML = '';
            isNextReady = true;
        }

        document.getElementById('bar-unflipped').style.display = 'block';
        document.getElementById('bar-flipped').style.display = 'none';

        // MathJax Pipeline Typesetting Queue
        if (window.MathJax && window.MathJax.Hub && window.MathJax.Hub.Queue) {
            // Priority 1: Typeset visible Card A Front
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, slotAFront]);

            // Priority 2: Typeset offscreen Card A Back
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, slotABack], function () {
                isTypesettingDone = true;
                if (pendingFlip) {
                    executeFlip();
                }
            });

            // Priority 3: Pre-typeset offscreen Card B (Front & Back)
            if (nextCardData) {
                window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, slotBFront]);
                window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, slotBBack], function () {
                    isNextReady = true;
                    if (pendingNext) {
                        executeNextSwap();
                    }
                });
            }
        } else {
            isTypesettingDone = true;
            isNextReady = true;
        }
    }

    // 3. Flip Card (Show Answer)
    function flipCard() {
        if (isFlipped) return;

        if (!isTypesettingDone) {
            console.log("[RAnki-SPA] Back answer typesetting in progress. Queue Lock engaged.");
            pendingFlip = true;
            return;
        }

        executeFlip();
    }

    function executeFlip() {
        if (currentGroup === 'A') {
            slotAFront.className = 'card-box offscreen';
            slotABack.className = 'card-box';
        } else {
            slotBFront.className = 'card-box offscreen';
            slotBBack.className = 'card-box';
        }

        isFlipped = true;
        pendingFlip = false;

        document.getElementById('bar-unflipped').style.display = 'none';
        document.getElementById('bar-flipped').style.display = 'block';

        if (cardAreaEl) {
            cardAreaEl.scrollTop = 0;
        }

        sendHost('CARD_FLIPPED', {});
    }

    // 4. Rate Card & Instant Next-Card Transition (<= 5ms)
    function rateCard(rating) {
        if (!hasNextCard) {
            // Last card completed: show celebration panel
            sendHost('RATE_CARD', { rating: rating });
            showCelebration();
            return;
        }

        if (!isNextReady) {
            console.log("[RAnki-SPA] Next card background typesetting in progress. Queue Lock engaged.");
            pendingNext = true;
            pendingRating = rating;
            return;
        }

        executeNextSwap(rating);
    }

    function executeNextSwap(rating) {
        var r = rating || pendingRating;
        pendingNext = false;
        pendingRating = '';

        // Instant DOM swap (<= 5ms)
        if (currentGroup === 'A') {
            currentGroup = 'B';
            slotAFront.className = 'card-box offscreen';
            slotABack.className = 'card-box offscreen';
            slotBFront.className = 'card-box';
            slotBBack.className = 'card-box offscreen';
        } else {
            currentGroup = 'A';
            slotBFront.className = 'card-box offscreen';
            slotBBack.className = 'card-box offscreen';
            slotAFront.className = 'card-box';
            slotABack.className = 'card-box offscreen';
        }

        currentCardData = nextCardData;
        nextCardData = null;
        isFlipped = false;
        pendingFlip = false;
        isTypesettingDone = true; // Back was already pre-typeset in offscreen buffer!
        isNextReady = false;

        document.getElementById('bar-unflipped').style.display = 'block';
        document.getElementById('bar-flipped').style.display = 'none';

        if (cardAreaEl) {
            cardAreaEl.scrollTop = 0;
        }

        if (currentCardData) {
            updateIntervals(currentCardData.intervals);
            if (styleEl && currentCardData.css) {
                styleEl.textContent = currentCardData.css;
            }
        }

        // Notify Host to commit review & fetch Card N+2
        sendHost('RATE_AND_FETCH_NEXT', { rating: r });
    }

    // 5. Handle PREPARE_NEXT_CARD from Host
    function handlePrepareNextCard(payload) {
        if (!payload || !payload.has_next || !payload.next_card) {
            hasNextCard = false;
            isNextReady = true;
            return;
        }

        nextCardData = payload.next_card;
        hasNextCard = true;
        isNextReady = false;

        // Populate the idle group
        var idleFront = (currentGroup === 'A') ? slotBFront : slotAFront;
        var idleBack = (currentGroup === 'A') ? slotBBack : slotABack;

        idleFront.className = 'card-box offscreen';
        idleBack.className = 'card-box offscreen';

        idleFront.innerHTML = preprocessCardHtml(nextCardData.question_html || '');
        idleBack.innerHTML = preprocessCardHtml(nextCardData.answer_html || '');

        // Background MathJax typesetting
        if (window.MathJax && window.MathJax.Hub && window.MathJax.Hub.Queue) {
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, idleFront]);
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, idleBack], function () {
                isNextReady = true;
                if (pendingNext) {
                    executeNextSwap();
                }
            });
        } else {
            isNextReady = true;
            if (pendingNext) {
                executeNextSwap();
            }
        }
    }

    // 6. Single-card Fallback
    function renderCardFallback(data) {
        renderCardPipeline({
            current: data,
            next: null,
            deck_name: '复习',
            new_count: 0,
            learn_count: 0,
            review_count: 0
        });
    }

    // 7. Deck Finished Celebration
    function showCelebration() {
        slotAFront.className = 'card-box offscreen';
        slotABack.className = 'card-box offscreen';
        slotBFront.className = 'card-box offscreen';
        slotBBack.className = 'card-box offscreen';

        if (panelFinished) {
            panelFinished.style.display = 'block';
        }
        document.getElementById('bar-unflipped').style.display = 'none';
        document.getElementById('bar-flipped').style.display = 'none';

        var titleEl = document.getElementById('deck-title');
        if (titleEl) titleEl.textContent = '今日复习已完成';
    }

    function exitApp() {
        sendHost('exit', {});
    }

    function backToDecks() {
        sendHost('EXIT_DECK', {});
    }

    // Setup events, scrolling & gestures on window load
    window.onload = function () {
        slotAFront = document.getElementById('slot-a-front');
        slotABack = document.getElementById('slot-a-back');
        slotBFront = document.getElementById('slot-b-front');
        slotBBack = document.getElementById('slot-b-back');

        styleEl = document.getElementById('card-dynamic-style');
        cardAreaEl = document.querySelector('.card-area');
        panelFinished = document.getElementById('panel-finished');

        // Touch & Mouse Drag Scrolling for Kindle WebKit 1.0
        if (cardAreaEl) {
            var touchStartY = 0;
            var touchStartScrollTop = 0;
            var isTouching = false;
            var touchMoved = false;

            cardAreaEl.addEventListener('touchstart', function (e) {
                if (e.touches && e.touches.length > 0) {
                    isTouching = true;
                    touchMoved = false;
                    touchStartY = e.touches[0].pageY;
                    touchStartScrollTop = cardAreaEl.scrollTop;
                }
            }, false);

            cardAreaEl.addEventListener('touchmove', function (e) {
                if (!isTouching || !e.touches || e.touches.length === 0) return;
                var deltaY = e.touches[0].pageY - touchStartY;
                if (Math.abs(deltaY) > 10) {
                    touchMoved = true;
                }
                cardAreaEl.scrollTop = touchStartScrollTop - deltaY;
            }, false);

            cardAreaEl.addEventListener('touchend', function (e) {
                isTouching = false;
                // If pure tap without drag, support 1/3 screen step paging
                if (!touchMoved && e.changedTouches && e.changedTouches.length > 0) {
                    var tapY = e.changedTouches[0].clientY;
                    var winHeight = window.innerHeight || 1448;
                    if (tapY < winHeight * 0.35) {
                        // Top 35%: scroll up 240px
                        cardAreaEl.scrollTop -= 240;
                    } else if (tapY > winHeight * 0.55 && tapY < winHeight * 0.90) {
                        // Bottom 55-90%: scroll down 240px
                        cardAreaEl.scrollTop += 240;
                    }
                }
            }, false);

            // Mouse fallback for X11 events
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

        var btnFinishedBack = document.getElementById('btn-finished-back');
        if (btnFinishedBack) {
            btnFinishedBack.onclick = backToDecks;
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
