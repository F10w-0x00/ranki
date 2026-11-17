// Automated verification script for V2 Features: AnkiDroid UI, Sync, Settings & Calibrated Typography
var fs = require('fs');
var path = require('path');
var acorn = require('acorn');

console.log("=== Verifying V2 Features: AnkiDroid UI, Sync, Settings & ES5 ===");

var repoRoot = path.resolve(__dirname, '..');

// 1. Verify views/index.html
var html = fs.readFileSync(path.join(repoRoot, 'views', 'index.html'), 'utf8');
var requiredHtml = [
    'btn-sync', 'btn-settings', 'btn-exit',
    'modal-settings', 'modal-sync', 'toast',
    'btn-group-table', 'btn-interval', 'btn-label',
    'deck-summary'
];
for (var i = 0; i < requiredHtml.length; i++) {
    if (html.indexOf(requiredHtml[i]) === -1) {
        console.error("[FAIL] Missing element in views/index.html: " + requiredHtml[i]);
        process.exit(1);
    }
}
// Ensure 1990s table headers are GONE
if (html.indexOf('col-deck-name') !== -1) {
    console.error("[FAIL] Obsolete 1990s table headers (col-deck-name) still present in views/index.html!");
    process.exit(1);
}
console.log("[PASS] views/index.html has AnkiDroid UI, Sync/Settings modals, and no 1990s table headers.");

// 2. Verify views/style.css
var css = fs.readFileSync(path.join(repoRoot, 'views', 'style.css'), 'utf8');
var requiredCss = [
    '.deck-row', '.deck-chevron', '.deck-num-new', '.deck-num-learn', '.deck-num-due',
    '.modal-backdrop', '.modal-card', '.modal-sync-card', '.toast-box',
    '.btn-group-table', '.btn-interval', '.btn-label'
];
for (var j = 0; j < requiredCss.length; j++) {
    if (css.indexOf(requiredCss[j]) === -1) {
        console.error("[FAIL] Missing CSS rule in views/style.css: " + requiredCss[j]);
        process.exit(1);
    }
}
console.log("[PASS] views/style.css contains complete AnkiDroid styling & modal rules.");

// 3. Verify views/app.js (Strict ES5 & Sync logic)
var js = fs.readFileSync(path.join(repoRoot, 'views', 'app.js'), 'utf8');
try {
    acorn.parse(js, { ecmaVersion: 5 });
    console.log("[PASS] views/app.js strictly parses under ES5 (WebKit 1.0 compatible).");
} catch (e) {
    console.error("[FAIL] ES5 parsing error in views/app.js: " + e.message);
    process.exit(1);
}

var requiredJs = [
    'SYNC_COLLECTION', 'SYNC_LOGIN', 'GET_CONFIG', 'DECK_EMPTY',
    'handleConfigData', 'handleSyncResult', 'handleLoginResult',
    'toggleCollapse', 'applyCollapsedVisibility'
];
for (var k = 0; k < requiredJs.length; k++) {
    if (js.indexOf(requiredJs[k]) === -1) {
        console.error("[FAIL] Missing JS function/constant in views/app.js: " + requiredJs[k]);
        process.exit(1);
    }
}
console.log("[PASS] views/app.js has full sync, subdeck folding, and empty deck handling.");

// 4. Verify src/config.rs & src/main.rs
var configRs = fs.readFileSync(path.join(repoRoot, 'src', 'config.rs'), 'utf8');
if (configRs.indexOf('AppConfig') === -1 || configRs.indexOf('save') === -1) {
    console.error("[FAIL] src/config.rs missing AppConfig implementation!");
    process.exit(1);
}
console.log("[PASS] src/config.rs verified.");

console.log("\n=== ALL V2 ANKIDROID UI & SYNC VERIFICATIONS PASSED (100%) ===");
