// Global CSS theme for Tethys Log.
//
// Fonts:
//   Editor:     "Adwaita Mono" — monospace, code-adjacent feel
//   UI chrome:  "Cantarell"    — GNOME sans, distinct from editor text
//
// Design: deep navy background, slate text, visible borders, bold weights.
// No ghost buttons — every interactive element has a clear background.

pub const BASE: &str = r#"
window {
    background: rgba(28, 28, 28, 0.80);
}
window:focus {
    background: rgba(30, 30, 30, 0.84);
}

scrolledwindow {
    background: transparent;
    border: none;
}

/* ── Editor text ─────────────────────────────────────────────────────────── */

textview {
    background: transparent;
    border: none;
    color: #c0c0c0;
    caret-color: #d8d8d8;
    font-family: "Adwaita Mono", "Cascadia Mono", "JetBrains Mono", monospace;
    font-size: 11pt;
    font-weight: 400;
}
textview:focus { color: #c8c8c8; }

text {
    background: transparent;
    color: #c0c0c0;
    font-family: "Adwaita Mono", "Cascadia Mono", "JetBrains Mono", monospace;
    font-size: 11pt;
    font-weight: 400;
}
text selection {
    background: rgba(80, 130, 180, 0.36);
    color: #e8e8e8;
}

scrollbar              { opacity: 0.10; }
scrollbar:hover        { opacity: 0.24; }
.tab-row scrollbar     { opacity: 0; min-width: 0; min-height: 0; }

/* ── Tab bar ─────────────────────────────────────────────────────────────── */

.tab-row {
    background: rgba(22, 22, 22, 0.94);
    border-bottom: 2px solid rgba(255,255,255,0.07);
    padding: 5px 4px 0 4px;
}
window:focus .tab-row {
    background: rgba(24, 24, 24, 0.97);
    border-bottom-color: rgba(255,255,255,0.10);
}

.tab {
    background: transparent;
    border-radius: 8px 8px 0 0;
    border: 1px solid transparent;
    border-bottom: none;
    padding: 0;
    margin-right: 2px;
    min-width: 0;
}
.tab:hover       { background: rgba(255,255,255,0.05); }
.tab-active      { background: rgba(30,30,30,0.98); border-color: rgba(255,255,255,0.09); }
window:focus .tab-active { background: rgba(30,30,30,1.0); border-color: rgba(255,255,255,0.14); }

.tab-title {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #5a6470;
    font-size: 9.5pt;
    font-family: "Cantarell", sans-serif;
    font-weight: 600;
    padding: 7px 2px 7px 10px;
    min-width: 0;
}
.tab-active .tab-title              { color: #a8b4be; }
window:focus .tab-active .tab-title { color: #c0cad2; font-weight: 700; }
.tab-title:hover                    { color: #8898a4; background: transparent; }

.tab-close {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #3a444c;
    font-size: 11pt;
    padding: 4px 8px;
    min-width: 26px;
    min-height: 26px;
}
.tab-close:hover {
    color: #e07070;
    background: rgba(220,80,80,0.14);
    border-radius: 5px;
}

.tab-scroll-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #3a444c;
    font-size: 13pt;
    font-weight: 700;
    padding: 2px 8px;
    min-width: 24px;
}
.tab-scroll-btn:hover { color: #7a8fa0; }

.new-tab-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #50606c;
    font-size: 14pt;
    font-weight: 600;
    padding: 2px 12px;
    margin-bottom: 2px;
}
.new-tab-btn:hover              { color: #8aafcc; }
window:focus .new-tab-btn:hover { color: #a0c8e8; }

/* ── Status bar ──────────────────────────────────────────────────────────── */

/* Path label — hexpand squishes the text away from the buttons on the right */
.note-path-label {
    color: #3a7080;
    font-size: 9pt;
    font-family: "Cantarell", sans-serif;
    font-weight: 600;
    padding-bottom: 1px;
}
window:focus .note-path-label { color: #4a9ab0; }

/* Attach buttons — compact, in the status bar between path and vim pill */
.attach-inline-btn {
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 4px;
    box-shadow: none;
    color: #4a5a68;
    font-family: "Cantarell", sans-serif;
    font-size: 8.2pt;
    font-weight: 700;
    padding: 2px 10px;
    margin: 0 2px 3px 2px;
    min-height: 0;
}
.attach-inline-btn:hover {
    background: rgba(255,255,255,0.12);
    border-color: rgba(255,255,255,0.22);
    color: #8a9aaa;
}
.attach-inline-btn:active {
    background: rgba(255,255,255,0.08);
}

/* ── Tab context menu ────────────────────────────────────────────────────── */

.tab-menu { padding: 3px; }

.tab-menu-item {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #8a939b;
    font-size: 9.5pt;
    font-family: "Cantarell", sans-serif;
    font-weight: 500;
    padding: 4px 14px;
    border-radius: 4px;
    min-height: 0;
}
.tab-menu-item:hover { background: rgba(255,255,255,0.07); color: #b8c2ca; }

.rename-entry {
    font-family: "Cantarell", sans-serif;
    font-weight: 500;
    font-size: 10.5pt;
}
.rename-hint {
    color: #4a5561;
    font-size: 8.8pt;
    font-family: "Cantarell", sans-serif;
}

/* ── Tab accent stripes ──────────────────────────────────────────────────── */

.tab-accent-c0504a { border-left: 3px solid #c0504a; }
.tab-accent-c07a30 { border-left: 3px solid #c07a30; }
.tab-accent-b09a20 { border-left: 3px solid #b09a20; }
.tab-accent-3a8f60 { border-left: 3px solid #3a8f60; }
.tab-accent-2a8a8a { border-left: 3px solid #2a8a8a; }
.tab-accent-3a70b0 { border-left: 3px solid #3a70b0; }
.tab-accent-7a50a8 { border-left: 3px solid #7a50a8; }
.tab-accent-a04070 { border-left: 3px solid #a04070; }

.tab-active.tab-accent-c0504a { border-left-color: #e06060; }
.tab-active.tab-accent-c07a30 { border-left-color: #e09040; }
.tab-active.tab-accent-b09a20 { border-left-color: #d0b830; }
.tab-active.tab-accent-3a8f60 { border-left-color: #50b878; }
.tab-active.tab-accent-2a8a8a { border-left-color: #40b0b0; }
.tab-active.tab-accent-3a70b0 { border-left-color: #5090d8; }
.tab-active.tab-accent-7a50a8 { border-left-color: #9a70d0; }
.tab-active.tab-accent-a04070 { border-left-color: #c06090; }

/* ── Resize grip ─────────────────────────────────────────────────────────── */

.resize-grip {
    color: rgba(255,255,255,0.22);
    font-size: 10pt;
    padding: 2px 4px;
    min-width: 18px;
    min-height: 18px;
}
.resize-grip:hover { color: rgba(255,255,255,0.52); }

/* ── Image widget ────────────────────────────────────────────────────────── */

.image-widget {
    border: 1px solid rgba(255,255,255,0.07);
    border-radius: 6px;
}
"#;


pub const SEARCH: &str = r#"
.search-bar {
    background: rgba(22, 22, 22, 0.96);
    border-bottom: 1px solid rgba(255,255,255,0.07);
    padding: 4px 8px;
}

.search-entry {
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.11);
    border-radius: 5px;
    color: #9aa3ab;
    font-family: "Cantarell", sans-serif;
    font-weight: 500;
    font-size: 10pt;
    padding: 3px 10px;
    min-height: 0;
}
.search-entry:focus {
    border-color: rgba(100,160,210,0.40);
    color: #c4ccd4;
}

.search-nav-btn,
.search-action-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #4a5561;
    font-size: 9.5pt;
    font-family: "Cantarell", sans-serif;
    font-weight: 600;
    padding: 3px 8px;
    min-height: 0;
    border-radius: 4px;
}
.search-nav-btn:hover,
.search-action-btn:hover { background: rgba(255,255,255,0.07); color: #9aa3ab; }

.search-match-count {
    color: #3a4a55;
    font-size: 9pt;
    font-family: "Cantarell", sans-serif;
}

.tab-dirty-dot {
    color: #4a7a96;
    font-size: 14pt;
    padding: 0 4px 2px 0;
}
"#;


pub const VIM: &str = r#"
/* Vim mode status pill — NORMAL / INSERT / VISUAL.
   Status indicator only — not interactive (no hover, no pointer cursor). */

.vim-pill {
    font-family: "Cantarell", sans-serif;
    font-size: 8.0pt;
    font-weight: 800;
    letter-spacing: 0.08em;
    padding: 3px 10px;
    border-radius: 4px;
    min-width: 64px;
}

.vim-insert {
    color: #40b878;
    background: rgba(40,160,100,0.12);
    border: 1px solid rgba(40,160,100,0.24);
}
.vim-normal {
    color: #e09040;
    background: rgba(210,140,50,0.12);
    border: 1px solid rgba(210,140,50,0.24);
}
.vim-visual {
    color: #7ab0e8;
    background: rgba(80,140,210,0.14);
    border: 1px solid rgba(80,140,210,0.28);
}
"#;


// ATTACH constant kept for any legacy reference but is no longer a separate bar.
pub const ATTACH: &str = "";
