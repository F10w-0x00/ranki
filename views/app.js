/* RAnki Unified Web-SPA Frontend Engine (views/app.js)
 * Strict ES5 syntax for WebKit 1.0 (JavaScriptCore 2012)
 * Authentic AnkiDroid UI, Subdeck Folding, Cloud Sync, & Quad-Slot Ping-Pong Pre-Rendering
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
    var toastTimer = null;

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

    // Decks state & subdeck collapsing
    var cachedDecks = [];
    var collapsedDecks = {}; // full_name -> boolean

    // Send up-channel command to Rust Host
    function sendHost(action, payload) {
        var token = tokenCounter++;
        var payloadStr = typeof payload === 'string' ? payload : JSON.stringify(payload || {});
        document.title = "RA:" + action + ":" + token + ":" + payloadStr;
    }

    // Toast notification display
    function showToast(msg) {
        var toastEl = document.getElementById('toast');
        if (!toastEl) return;
        toastEl.textContent = msg;
        toastEl.style.display = 'block';
        if (toastTimer) {
            clearTimeout(toastTimer);
        }
        toastTimer = setTimeout(function () {
            toastEl.style.display = 'none';
        }, 3500);
    }

    // Down-channel dispatcher called by Rust Host: window.RAnki.dispatch(event, payload)
    window.RAnki = {
        dispatch: function (event, payload) {
            console.log("[RAnki-SPA] Received event: " + event);
            if (event === 'HOST_ACK') {
                console.log("[RAnki-SPA] Host handshake acknowledged.");
            } else if (event === 'LOAD_DECK_LIST') {
                renderDeckList(payload);
            } else if (event === 'DECK_EMPTY') {
                showToast(payload.message || '🎉 该牌组今日待复习卡片已全部完成！');
                if (payload.decks) {
                    renderDeckList(payload);
                }
            } else if (event === 'SHOW_CARD_PIPELINE') {
                renderCardPipeline(payload);
            } else if (event === 'PREPARE_NEXT_CARD') {
                handlePrepareNextCard(payload);
            } else if (event === 'SHOW_CARD') {
                renderCardFallback(payload);
            } else if (event === 'CONFIG_DATA') {
                handleConfigData(payload);
            } else if (event === 'LOGIN_RESULT') {
                handleLoginResult(payload);
            } else if (event === 'SYNC_RESULT') {
                handleSyncResult(payload);
            } else if (event === 'SETTINGS_SAVED') {
                showToast('✅ 设置已保存');
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

    // 1. Render AnkiDroid-Authentic Deck Browser (No Table Grid)
    function renderDeckList(data) {
        currentView = 'deck-picker';
        document.getElementById('deck-view').style.display = 'block';
        document.getElementById('review-view').style.display = 'none';

        // Top Header in Deck Mode
        var brandEl = document.getElementById('deck-header-brand');
        if (brandEl) brandEl.style.display = 'inline-block';

        var backBtn = document.getElementById('btn-back');
        if (backBtn) backBtn.style.display = 'none';

        var titleEl = document.getElementById('deck-title');
        if (titleEl) titleEl.style.display = 'none';

        var actionsEl = document.getElementById('deck-actions');
        if (actionsEl) actionsEl.style.display = 'inline-block';

        var badgesEl = document.getElementById('header-badges');
        if (badgesEl) badgesEl.style.display = 'none';

        var listEl = document.getElementById('deck-list');
        listEl.innerHTML = '';

        var decks = (data && data.decks) || [];
        cachedDecks = decks;

        if (decks.length === 0) {
            listEl.innerHTML = '<div class="empty-msg" style="padding: 40px; text-align: center; font-size: 28px;">没有可复习的牌组</div>';
            return;
        }

        var totalDue = 0;
        for (var j = 0; j < decks.length; j++) {
            totalDue += (decks[j].due_count || 0);
        }

        var subtitleEl = document.getElementById('deck-subtitle');
        if (subtitleEl) {
            subtitleEl.textContent = '待复习: ' + totalDue + ' 张';
        }

        var summaryEl = document.getElementById('deck-summary');
        if (summaryEl) {
            summaryEl.textContent = data.studied_today || ('今天在 0 秒内学习了 0 张卡片 (平均每张卡片 0 秒)');
        }

        for (var i = 0; i < decks.length; i++) {
            var deck = decks[i];
            var depth = deck.depth || 0;
            var isParent = !!deck.has_children;
            var indentPx = depth * 28;

            var row = document.createElement('div');
            row.className = 'deck-row';
            row.setAttribute('data-id', deck.id);
            row.setAttribute('data-full-name', deck.full_name);
            row.setAttribute('data-depth', depth);

            // Left side: chevron + deck name
            var leftDiv = document.createElement('div');
            leftDiv.className = 'deck-row-left ' + (isParent ? 'is-parent' : 'is-child');
            leftDiv.style.paddingLeft = indentPx + 'px';

            if (isParent) {
                var isCollapsed = !!collapsedDecks[deck.full_name];
                var chevron = document.createElement('span');
                chevron.className = 'deck-chevron';
                chevron.textContent = isCollapsed ? '▸' : '▾';
                chevron.setAttribute('data-target-full-name', deck.full_name);
                chevron.onclick = (function (fn, chevEl) {
                    return function (e) {
                        e.stopPropagation();
                        toggleCollapse(fn, chevEl);
                    };
                })(deck.full_name, chevron);
                leftDiv.appendChild(chevron);
            }

            var nameSpan = document.createElement('span');
            nameSpan.className = 'deck-name-text';
            nameSpan.textContent = deck.name;
            leftDiv.appendChild(nameSpan);

            // Right side: 3 numeric counts
            var rightDiv = document.createElement('div');
            rightDiv.className = 'deck-row-right';

            var newC = deck.new_count || 0;
            var learnC = deck.learn_count || 0;
            var dueC = deck.review_count || 0;

            var newSpan = document.createElement('span');
            newSpan.className = 'deck-num deck-num-new' + (newC === 0 ? ' zero' : '');
            newSpan.textContent = newC;

            var learnSpan = document.createElement('span');
            learnSpan.className = 'deck-num deck-num-learn' + (learnC === 0 ? ' zero' : '');
            learnSpan.textContent = learnC;

            var dueSpan = document.createElement('span');
            dueSpan.className = 'deck-num deck-num-due' + (dueC === 0 ? ' zero' : '');
            dueSpan.textContent = dueC;

            rightDiv.appendChild(newSpan);
            rightDiv.appendChild(learnSpan);
            rightDiv.appendChild(dueSpan);

            row.appendChild(leftDiv);
            row.appendChild(rightDiv);

            // Click deck row to select deck
            row.onclick = (function (dId) {
                return function () {
                    sendHost('SELECT_DECK', { deck_id: dId });
                };
            })(deck.id);

            listEl.appendChild(row);
        }

        // Apply collapsed state
        applyCollapsedVisibility();
    }

    // Toggle collapse for parent deck
    function toggleCollapse(fullName, chevronEl) {
        collapsedDecks[fullName] = !collapsedDecks[fullName];
        chevronEl.textContent = collapsedDecks[fullName] ? '▸' : '▾';
        applyCollapsedVisibility();
    }

    // Apply visibility based on collapsed parent decks
    function applyCollapsedVisibility() {
        var rows = document.querySelectorAll('.deck-row');
        for (var i = 0; i < rows.length; i++) {
            var row = rows[i];
            var fn = row.getAttribute('data-full-name') || '';
            var shouldHide = false;

            for (var pName in collapsedDecks) {
                if (collapsedDecks.hasOwnProperty(pName) && collapsedDecks[pName]) {
                    if (fn.indexOf(pName + '::') === 0) {
                        shouldHide = true;
                        break;
                    }
                }
            }

            row.style.display = shouldHide ? 'none' : 'block';
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

        // Top Header in Reviewer Mode
        var brandEl = document.getElementById('deck-header-brand');
        if (brandEl) brandEl.style.display = 'none';

        var backBtn = document.getElementById('btn-back');
        if (backBtn) backBtn.style.display = 'inline-block';

        var titleEl = document.getElementById('deck-title');
        if (titleEl) {
            titleEl.style.display = 'inline-block';
            titleEl.textContent = data.deck_name || '卡片复习';
        }

        var actionsEl = document.getElementById('deck-actions');
        if (actionsEl) actionsEl.style.display = 'none';

        var badgesEl = document.getElementById('header-badges');
        if (badgesEl) {
            badgesEl.style.display = 'inline-block';
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
        isFlipped = true;
        pendingFlip = false;

        if (currentGroup === 'A') {
            slotAFront.className = 'card-box offscreen';
            slotABack.className = 'card-box';
            slotABack.scrollTop = 0;
        } else {
            slotBFront.className = 'card-box offscreen';
            slotBBack.className = 'card-box';
            slotBBack.scrollTop = 0;
        }

        document.getElementById('bar-unflipped').style.display = 'none';
        document.getElementById('bar-flipped').style.display = 'block';

        sendHost('CARD_FLIPPED', {});
    }

    // 4. Rate Card & Advance Ping-Pong Buffer
    function handleRating(rating) {
        if (!isFlipped) return;

        if (!hasNextCard) {
            // Last card or deck completed
            sendHost('RATE_CARD', { rating: rating });
            return;
        }

        if (!isNextReady) {
            console.log("[RAnki-SPA] Next card typesetting in progress. Queue lock engaged.");
            pendingNext = true;
            pendingRating = rating;
            return;
        }

        executeNextSwap(rating);
    }

    function executeNextSwap(rating) {
        if (!rating && pendingRating) {
            rating = pendingRating;
            pendingRating = '';
        }
        pendingNext = false;

        // Instant DOM swap to pre-rendered card
        if (currentGroup === 'A') {
            slotABack.className = 'card-box offscreen';
            slotBFront.className = 'card-box';
            slotBFront.scrollTop = 0;
            currentGroup = 'B';
        } else {
            slotBBack.className = 'card-box offscreen';
            slotAFront.className = 'card-box';
            slotAFront.scrollTop = 0;
            currentGroup = 'A';
        }

        currentCardData = nextCardData;
        nextCardData = null;
        isFlipped = false;
        isTypesettingDone = true;
        isNextReady = false;

        updateIntervals(currentCardData ? currentCardData.intervals : null);
        document.getElementById('bar-unflipped').style.display = 'block';
        document.getElementById('bar-flipped').style.display = 'none';

        sendHost('RATE_AND_FETCH_NEXT', { rating: rating || 'good' });
    }

    // 5. Receive next card N+2 in pipeline
    function handlePrepareNextCard(data) {
        if (!data || !data.has_next || !data.next_card) {
            hasNextCard = false;
            isNextReady = true;
            console.log("[RAnki-SPA] Reached end of card queue.");
            return;
        }

        hasNextCard = true;
        nextCardData = data.next_card;

        var targetFront = (currentGroup === 'A') ? slotBFront : slotAFront;
        var targetBack = (currentGroup === 'A') ? slotBBack : slotABack;

        targetFront.className = 'card-box offscreen';
        targetBack.className = 'card-box offscreen';
        targetFront.innerHTML = preprocessCardHtml(nextCardData.question_html || '');
        targetBack.innerHTML = preprocessCardHtml(nextCardData.answer_html || '');

        isNextReady = false;

        if (window.MathJax && window.MathJax.Hub && window.MathJax.Hub.Queue) {
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, targetFront]);
            window.MathJax.Hub.Queue(["Typeset", window.MathJax.Hub, targetBack], function () {
                isNextReady = true;
                if (pendingNext) {
                    executeNextSwap();
                }
            });
        } else {
            isNextReady = true;
        }
    }

    // Fallback for single card payload
    function renderCardFallback(data) {
        renderCardPipeline({
            current: data,
            next: null,
            deck_name: '卡片复习',
            new_count: 0,
            learn_count: 0,
            review_count: 1
        });
    }

    // 6. Settings & Sync IPC Handlers
    function handleConfigData(payload) {
        var badge = document.getElementById('setting-status-badge');
        if (badge) {
            if (payload.has_hkey) {
                badge.textContent = '当前状态: 已配置 AnkiWeb 密钥 (可直接同步)';
                badge.style.color = '#005500';
            } else {
                badge.textContent = '当前状态: 未登录 (请输入账号密码登录)';
                badge.style.color = '#880000';
            }
        }
        var srvEl = document.getElementById('setting-server');
        if (srvEl && payload.sync_server) {
            srvEl.value = payload.sync_server;
        }
    }

    function handleLoginResult(payload) {
        document.getElementById('modal-sync').style.display = 'none';
        if (payload.success) {
            showToast('✅ ' + payload.message);
            // Automatically trigger sync upon successful login
            document.getElementById('modal-sync').style.display = 'block';
            sendHost('SYNC_COLLECTION', {});
        } else {
            showToast('❌ ' + payload.message);
            document.getElementById('modal-settings').style.display = 'block';
        }
    }

    function handleSyncResult(payload) {
        document.getElementById('modal-sync').style.display = 'none';
        if (payload.success) {
            showToast('✅ ' + payload.message);
            if (payload.decks) {
                renderDeckList({
                    decks: payload.decks,
                    studied_today: payload.studied_today
                });
            }
        } else {
            showToast('❌ ' + payload.message);
            if (payload.need_login) {
                document.getElementById('modal-settings').style.display = 'block';
                sendHost('GET_CONFIG', {});
            }
        }
    }

    // 7. Page Initialization
    window.addEventListener('DOMContentLoaded', function () {
        slotAFront = document.getElementById('slot-a-front');
        slotABack = document.getElementById('slot-a-back');
        slotBFront = document.getElementById('slot-b-front');
        slotBBack = document.getElementById('slot-b-back');
        styleEl = document.getElementById('card-dynamic-style');
        cardAreaEl = document.querySelector('.card-area');
        panelFinished = document.getElementById('panel-finished');

        // Top Bar Navigation
        var backBtn = document.getElementById('btn-back');
        if (backBtn) {
            backBtn.onclick = function () {
                sendHost('EXIT_DECK', {});
            };
        }

        var exitBtn = document.getElementById('btn-exit');
        if (exitBtn) {
            exitBtn.onclick = function () {
                sendHost('exit', {});
            };
        }

        var syncBtn = document.getElementById('btn-sync');
        if (syncBtn) {
            syncBtn.onclick = function () {
                document.getElementById('modal-sync').style.display = 'block';
                sendHost('SYNC_COLLECTION', {});
            };
        }

        var settingsBtn = document.getElementById('btn-settings');
        if (settingsBtn) {
            settingsBtn.onclick = function () {
                document.getElementById('modal-settings').style.display = 'block';
                sendHost('GET_CONFIG', {});
            };
        }

        // Settings Modal Buttons
        var modalCloseBtn = document.getElementById('btn-modal-close');
        if (modalCloseBtn) {
            modalCloseBtn.onclick = function () {
                document.getElementById('modal-settings').style.display = 'none';
            };
        }

        var modalSaveBtn = document.getElementById('btn-modal-save');
        if (modalSaveBtn) {
            modalSaveBtn.onclick = function () {
                var srv = document.getElementById('setting-server').value || '';
                sendHost('SAVE_SETTINGS', { sync_server: srv });
                document.getElementById('modal-settings').style.display = 'none';
            };
        }

        var modalLoginBtn = document.getElementById('btn-modal-login');
        if (modalLoginBtn) {
            modalLoginBtn.onclick = function () {
                var u = document.getElementById('setting-user').value || '';
                var p = document.getElementById('setting-pass').value || '';
                var srv = document.getElementById('setting-server').value || '';
                if (!u || !p) {
                    showToast('⚠️ 账号和密码不能为空');
                    return;
                }
                document.getElementById('modal-settings').style.display = 'none';
                document.getElementById('modal-sync').style.display = 'block';
                sendHost('SYNC_LOGIN', { username: u, password: p, sync_server: srv });
            };
        }

        // Show Answer Button
        var btnShowAnswer = document.getElementById('btn-show-answer');
        if (btnShowAnswer) {
            btnShowAnswer.onclick = function () {
                flipCard();
            };
        }

        // Rating Buttons
        var btnAgain = document.getElementById('btn-again');
        if (btnAgain) {
            btnAgain.onclick = function () { handleRating('again'); };
        }
        var btnHard = document.getElementById('btn-hard');
        if (btnHard) {
            btnHard.onclick = function () { handleRating('hard'); };
        }
        var btnGood = document.getElementById('btn-good');
        if (btnGood) {
            btnGood.onclick = function () { handleRating('good'); };
        }
        var btnEasy = document.getElementById('btn-easy');
        if (btnEasy) {
            btnEasy.onclick = function () { handleRating('easy'); };
        }

        var btnFinishedBack = document.getElementById('btn-finished-back');
        if (btnFinishedBack) {
            btnFinishedBack.onclick = function () {
                sendHost('EXIT_DECK', {});
            };
        }

        // Screen Top/Bottom 1/3 Tap Paging
        if (cardAreaEl) {
            cardAreaEl.onclick = function (e) {
                if (e.target && (e.target.tagName === 'BUTTON' || e.target.closest('button'))) {
                    return;
                }
                var activeBox = (currentGroup === 'A')
                    ? (isFlipped ? slotABack : slotAFront)
                    : (isFlipped ? slotBBack : slotBFront);

                if (!activeBox) return;

                var clientY = e.clientY;
                var height = window.innerHeight || 1448;
                if (clientY > height * 0.67) {
                    activeBox.scrollTop += 360;
                } else if (clientY < height * 0.33) {
                    activeBox.scrollTop -= 360;
                }
            };
        }

        // Handshake with Rust Host
        sendHost('ready', {});
    });
})();
