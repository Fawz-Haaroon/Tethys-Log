// Rich text markup and syntax highlighting for the note TextBuffer.
//
// ── Two independent passes ────────────────────────────────────────────────────
//
// Pass 1 — Code blocks  (```lang … ```)
//   Scans for fenced code blocks and applies per-token colour tags inside
//   each block.  The opening ``` and lang hint are concealed (invisible) so the
//   block appears clean.  When the cursor moves onto a fence line, that line is
//   revealed — Neovim-style concealment via the `syn-concealed` tag.
//
// Pass 2 — Markdown markup  (outside code blocks)
//   Headings, bold, italic, inline code, horizontal rules, blockquotes, lists.
//   Code-block regions are skipped.
//
// Debounce: 400 ms after every buffer change before re-running both passes.
// Fence concealment updates immediately on every cursor-position change.

use std::{cell::RefCell, rc::Rc, time::Duration};

use gtk::{glib, prelude::*, TextBuffer, TextView};

// ── Enhanced Tokyo Night palette ──────────────────────────────────────────────
const KEYWORD:   &str = "#c678dd";
const STRING:    &str = "#98c379";
const COMMENT:   &str = "#7f848e";
const NUMBER:    &str = "#d19a66";
const TYPE_COL:  &str = "#56b6c2";
const FUNCTION:  &str = "#61afef";
const OPERATOR:  &str = "#89ddff";
const MACRO_COL: &str = "#e5c07b";
const CONST_COL: &str = "#f0a070";
const DECO_COL:  &str = "#e06c75";
const CODE_BG:   &str = "#101820";
const LANG_BG:   &str = "#0c1520";
const LANG_FG:   &str = "#3d7a9a";

// ── keyword tables ────────────────────────────────────────────────────────────
const RUST_KW: &[&str] = &[
    "as","async","await","break","const","continue","crate","dyn",
    "else","enum","extern","false","fn","for","if","impl","in",
    "let","loop","match","mod","move","mut","pub","ref","return",
    "self","Self","static","str","struct","super","trait","true","type",
    "unsafe","use","where","while","i8","i16","i32","i64","i128","isize",
    "u8","u16","u32","u64","u128","usize","f32","f64","bool","char",
];
const PY_KW: &[&str] = &[
    "False","None","True","and","as","assert","async","await",
    "break","class","continue","def","del","elif","else","except",
    "finally","for","from","global","if","import","in","is","lambda",
    "nonlocal","not","or","pass","raise","return","try","while","with","yield",
];
const JS_KW: &[&str] = &[
    "async","await","break","case","catch","class","const","continue",
    "debugger","default","delete","do","else","export","extends","false",
    "finally","for","from","function","if","import","in","instanceof",
    "let","new","null","of","return","static","super","switch","this",
    "throw","true","try","typeof","undefined","var","void","while","with","yield",
    "number","string","boolean","object","any","never","unknown",
];
const GO_KW: &[&str] = &[
    "break","case","chan","const","continue","default","defer","else",
    "fallthrough","for","func","go","goto","if","import","interface",
    "map","package","range","return","select","struct","switch","type","var",
    "error","string","bool","byte","rune","int","int8","int16","int32","int64",
    "uint","uint8","uint16","uint32","uint64","uintptr","float32","float64",
];
const GENERIC_KW: &[&str] = &[
    "if","else","for","while","do","return","break","continue",
    "true","false","null","nil","undefined","void","let","var","const",
    "func","function","class","new","this","self","import","export","from",
    "use","pub","fn","def","end","begin","in","of","is","not","and","or",
];

fn keywords_for(lang: &str) -> &'static [&'static str] {
    match lang {
        "rust" | "rs"                             => RUST_KW,
        "python" | "py"                           => PY_KW,
        "js" | "javascript" | "ts" | "typescript" => JS_KW,
        "go"                                      => GO_KW,
        _                                         => GENERIC_KW,
    }
}

fn is_hash_comment_lang(lang: &str) -> bool {
    matches!(lang, "python"|"py"|"sh"|"bash"|"shell"|"zsh"|"ruby"|"rb"|"r"|"perl"|"pl")
}

fn is_rust_lang(lang: &str) -> bool {
    matches!(lang, "rust"|"rs")
}

// ── tag registration ──────────────────────────────────────────────────────────
fn register_tags(buffer: &TextBuffer) {
    let tt = buffer.tag_table();

    // Code block syntax tags
    let syn_defs: &[(&str, &str, &str)] = &[
        ("syn-keyword",   "foreground", KEYWORD),
        ("syn-string",    "foreground", STRING),
        ("syn-comment",   "foreground", COMMENT),
        ("syn-number",    "foreground", NUMBER),
        ("syn-type",      "foreground", TYPE_COL),
        ("syn-function",  "foreground", FUNCTION),
        ("syn-operator",  "foreground", OPERATOR),
        ("syn-macro",     "foreground", MACRO_COL),
        ("syn-const",     "foreground", CONST_COL),
        ("syn-decorator", "foreground", DECO_COL),
    ];
    for (name, prop, val) in syn_defs {
        if tt.lookup(name).is_none() {
            if let Some(t) = buffer.create_tag(Some(name), &[]) {
                t.set_property(*prop, *val);
                if *name == "syn-comment" {
                    t.set_property("style", gtk::pango::Style::Italic);
                }
                if *name == "syn-macro" || *name == "syn-const" {
                    t.set_property("weight", 700_i32);
                }
            }
        }
    }

    // Code block background
    if tt.lookup("syn-codebg").is_none() {
        if let Some(t) = buffer.create_tag(Some("syn-codebg"), &[]) {
            t.set_property("background",           CODE_BG);
            t.set_property("paragraph-background", CODE_BG);
        }
    }
    // Fence header line (``` lang)
    if tt.lookup("syn-codelang").is_none() {
        if let Some(t) = buffer.create_tag(Some("syn-codelang"), &[]) {
            t.set_property("background",           LANG_BG);
            t.set_property("paragraph-background", LANG_BG);
            t.set_property("foreground",           LANG_FG);
            t.set_property("weight",               600_i32);
        }
    }
    // Fence concealment: makes the ``` backticks invisible (Neovim-style).
    // Removed from cursor's current line by update_fence_concealment().
    if tt.lookup("syn-concealed").is_none() {
        if let Some(t) = buffer.create_tag(Some("syn-concealed"), &[]) {
            t.set_property("invisible", true);
        }
    }

    // Heading hierarchy via weight + colour only.
    // GTK TextView's Pango layout treats `scale` and `size-points` as
    // per-character font metrics; when a heading tag ends mid-visual-line
    // (wrap boundary or partial application) the larger ascent/descent
    // contaminates neighbour characters' vertical placement, causing the
    // "some letters bigger than others" bleed.  Weight + colour carry the
    // visual hierarchy without touching font metrics at all.
    let md_defs: &[(&str, &[(&str, &str)])] = &[
        ("md-h1", &[
            ("foreground", "#e8f0f8"),
            ("weight",     "800"),
        ]),
        ("md-h2", &[
            ("foreground", "#ccdae8"),
            ("weight",     "700"),
        ]),
        ("md-h3", &[
            ("foreground", "#b0c2d4"),
            ("weight",     "600"),
        ]),
        ("md-bold", &[
            ("foreground", "#d8e2ea"),
            ("weight",     "700"),
        ]),
        ("md-italic", &[]),
        ("md-code", &[
            ("background", "#182230"),
            ("foreground", "#7dcfff"),
            ("family",     "monospace"),
        ]),
        ("md-hr", &[
            ("foreground", "#2e3e50"),
            ("weight",     "700"),
        ]),
        ("md-quote", &[
            ("foreground",  "#6a7a86"),
            ("left-margin", "24"),
        ]),
        ("md-list", &[
            ("left-margin", "18"),
        ]),
        ("md-strikethrough", &[
            ("foreground",   "#5a6a7a"),
            ("strikethrough","1"),
        ]),
    ];

    for (name, props) in md_defs {
        if tt.lookup(name).is_none() {
            if let Some(t) = buffer.create_tag(Some(name), &[]) {
                for (prop, val) in *props {
                    match *prop {
                        "weight" => {
                            let w: i32 = val.parse().unwrap_or(400);
                            t.set_property("weight", w);
                        }
                        "left-margin" => {
                            let m: i32 = val.parse().unwrap_or(0);
                            t.set_property("left-margin", m);
                        }
                        "pixels-above-lines" => {
                            let n: i32 = val.parse().unwrap_or(0);
                            t.set_property("pixels-above-lines", n);
                        }
                        "scale" => {
                            let s: f64 = val.parse().unwrap_or(1.0);
                            t.set_property("scale", s);
                        }
                        "strikethrough" => {
                            t.set_property("strikethrough", true);
                        }
                        _ => { t.set_property(*prop, *val); }
                    }
                }
                if *name == "md-italic" || *name == "md-quote" {
                    t.set_property("style", gtk::pango::Style::Italic);
                }
            }
        }
    }
}

// ── public entry point ────────────────────────────────────────────────────────
pub fn wire_syntax_highlighting(view: &TextView) {
    let buffer = view.buffer();
    register_tags(&buffer);

    // ── debounced pass: code-block backgrounds only ───────────────────────────
    // Syntax colouring, markdown markup, and fence concealment are stalled —
    // all three have an unresolved offset-drift bug under gtk4-rs 0.11 that
    // makes tags land on the wrong characters.  The only reliable operation
    // remaining is applying background colours to fenced code blocks, which
    // is done here.  The other passes are preserved below as dead code so
    // they can be re-enabled once the root cause is identified.
    let pending:    Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let buf_ref     = buffer.clone();
    let pending_ref = pending.clone();

    buffer.connect_changed(move |_| {
        if let Some(id) = pending_ref.borrow_mut().take() { id.remove(); }
        let buf2     = buf_ref.clone();
        let pending2 = pending_ref.clone();
        let src = glib::timeout_add_local(Duration::from_millis(400), move || {
            apply_highlights(&buf2);
            *pending2.borrow_mut() = None;
            glib::ControlFlow::Break
        });
        *pending_ref.borrow_mut() = Some(src);
    });

    // Initial pass
    apply_highlights(&buffer);
}


// ── top-level highlight pass ──────────────────────────────────────────────────
// Applies only code-block background colours.  Syntax tokenisation, fence
// concealment, and markdown markup are stalled — see wire_syntax_highlighting.
fn apply_highlights(buffer: &TextBuffer) {
    let start = buffer.start_iter();
    let end   = buffer.end_iter();

    // Clear only the background tags we manage; leave everything else alone.
    for name in ["syn-codebg", "syn-codelang"] {
        if let Some(tag) = buffer.tag_table().lookup(name) {
            buffer.remove_tag(&tag, &start, &end);
        }
    }

    let text: String = buffer.text(&start, &end, true).to_string();
    let chars: Vec<char> = text.chars().collect();

    let code_blocks = find_code_blocks(&chars);

    for &(block_start, lang_end, content_start, block_end) in &code_blocks {
        let open_line_end = (lang_end + 1).min(chars.len());

        // Opening fence line: dim header background + muted lang-hint colour.
        apply_tag(buffer, "syn-codelang", block_start as i32, open_line_end as i32);

        // Content + closing fence: dark code background (covers empty lines too).
        apply_tag(buffer, "syn-codebg", content_start as i32, block_end as i32);
    }
}


// ── fence concealment: cursor-driven reveal ───────────────────────────────────
// Removes syn-concealed from whichever fence line the cursor is on (if any),
// so the ``` chars are visible when editing that line.
fn update_fence_concealment(
    buffer: &TextBuffer,
    blocks: &[(usize, usize, usize, usize)],
) {
    let concealed_tag = match buffer.tag_table().lookup("syn-concealed") {
        Some(t) => t,
        None    => return,
    };

    let cursor_iter = buffer.iter_at_mark(&buffer.get_insert());
    let cursor_line = cursor_iter.line();

    for &(block_start, _lang_end, _content_start, block_end) in blocks {
        // Opening fence: only the 3 backtick characters.
        // Must match the range applied in apply_highlights — never include the
        // newline or lang hint, to avoid the GTK invisible-newline paragraph bug.
        let open_iter     = buffer.iter_at_offset(block_start as i32);
        let open_tick_end = buffer.iter_at_offset((block_start + 3) as i32);
        let open_line     = open_iter.line();

        if open_line == cursor_line {
            buffer.remove_tag(&concealed_tag, &open_iter, &open_tick_end);
        } else {
            buffer.apply_tag(&concealed_tag, &open_iter, &open_tick_end);
        }

        // Closing fence: block_end-3 .. block_end
        if block_end >= 3 {
            let close_start = buffer.iter_at_offset((block_end - 3) as i32);
            let close_end   = buffer.iter_at_offset(block_end as i32);
            let close_line  = close_start.line();

            if close_line == cursor_line {
                buffer.remove_tag(&concealed_tag, &close_start, &close_end);
            } else {
                buffer.apply_tag(&concealed_tag, &close_start, &close_end);
            }
        }
    }
}


// ── code block detection ──────────────────────────────────────────────────────
// Returns vec of (block_start, lang_end, content_start, block_end)
fn find_code_blocks(chars: &[char]) -> Vec<(usize, usize, usize, usize)> {
    let mut blocks = Vec::new();
    let len = chars.len();
    let mut pos = 0;

    while pos + 2 < len {
        if chars[pos] == '`' && chars[pos+1] == '`' && chars[pos+2] == '`' {
            let block_start = pos;
            pos += 3;
            // skip lang hint (up to newline)
            while pos < len && chars[pos] != '\n' { pos += 1; }
            let lang_end = pos;
            if pos < len { pos += 1; } // skip newline
            let content_start = pos;
            // find closing ```
            let before = blocks.len();
            while pos + 2 < len {
                if chars[pos] == '`' && chars[pos+1] == '`' && chars[pos+2] == '`' {
                    let block_end = pos + 3;
                    blocks.push((block_start, lang_end, content_start, block_end));
                    pos += 3;
                    break;
                }
                pos += 1;
            }
            if blocks.len() == before {
                break; // unclosed fence
            }
        } else {
            pos += 1;
        }
    }
    blocks
}

fn in_code_block(blocks: &[(usize, usize, usize, usize)], pos: usize) -> bool {
    blocks.iter().any(|&(s, _, _, e)| pos >= s && pos < e)
}


// ── markdown highlighter ──────────────────────────────────────────────────────
fn highlight_markdown(
    buffer: &TextBuffer,
    chars:  &[char],
    code_blocks: &[(usize, usize, usize, usize)],
) {
    let len = chars.len();
    let mut pos = 0;

    while pos < len {
        let line_start = pos;
        while pos < len && chars[pos] != '\n' { pos += 1; }
        let line_end = pos;
        if pos < len { pos += 1; }

        if in_code_block(code_blocks, line_start) { continue; }
        if line_start >= line_end { continue; }

        // ── Heading ────────────────────────────────────────────────────────
        let pound_count = chars[line_start..line_end]
            .iter().take_while(|&&c| c == '#').count();
        if pound_count > 0 && pound_count <= 3
            && line_start + pound_count < line_end
            && chars[line_start + pound_count] == ' '
        {
            let tag = match pound_count { 1 => "md-h1", 2 => "md-h2", _ => "md-h3" };
            apply_tag(buffer, tag, line_start as i32, line_end as i32);
            continue;
        }

        // ── Horizontal rule: --- / *** / ___ ──────────────────────────────
        {
            let line_str: String = chars[line_start..line_end].iter().collect();
            let t = line_str.trim();
            if t == "---" || t == "***" || t == "___" {
                apply_tag(buffer, "md-hr", line_start as i32, line_end as i32);
                continue;
            }

            // ── Blockquote ─────────────────────────────────────────────────
            if t.starts_with("> ") || t == ">" {
                apply_tag(buffer, "md-quote", line_start as i32, line_end as i32);
                continue;
            }

            // ── List item ──────────────────────────────────────────────────
            if t.starts_with("- ")  || t.starts_with("* ")  ||
               t.starts_with("+ ")  ||
               (t.len() > 2 && t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && t.contains(". "))
            {
                apply_tag(buffer, "md-list", line_start as i32, line_end as i32);
            }
        }

        // ── Inline elements ────────────────────────────────────────────────
        let mut p = line_start;
        while p < line_end {
            let c = chars[p];

            // Inline code: `...`
            if c == '`' {
                let s = p; p += 1;
                while p < line_end && chars[p] != '`' { p += 1; }
                if p < line_end { p += 1; }
                if p > s + 1 { apply_tag(buffer, "md-code", s as i32, p as i32); }
                continue;
            }

            // Bold: **...**
            if c == '*' && p + 1 < line_end && chars[p+1] == '*' {
                let s = p; p += 2;
                while p + 1 < line_end && !(chars[p] == '*' && chars[p+1] == '*') { p += 1; }
                if p + 1 < line_end { p += 2; }
                if p > s + 4 { apply_tag(buffer, "md-bold", s as i32, p as i32); }
                continue;
            }

            // Italic: *word* (at least 3 chars, not double-star)
            if c == '*' && p + 2 < line_end && chars[p+1] != '*' {
                let s = p; p += 1;
                while p < line_end && chars[p] != '*' { p += 1; }
                if p < line_end { p += 1; }
                // require content (s+1 .. p-1) to be non-empty
                if p > s + 2 { apply_tag(buffer, "md-italic", s as i32, p as i32); }
                continue;
            }

            // Italic: _word_
            if c == '_' && p + 2 < line_end && chars[p+1] != '_' {
                let s = p; p += 1;
                while p < line_end && chars[p] != '_' { p += 1; }
                if p < line_end { p += 1; }
                if p > s + 2 { apply_tag(buffer, "md-italic", s as i32, p as i32); }
                continue;
            }

            // Strikethrough: ~~text~~
            if c == '~' && p + 1 < line_end && chars[p+1] == '~' {
                let s = p; p += 2;
                while p + 1 < line_end && !(chars[p] == '~' && chars[p+1] == '~') { p += 1; }
                if p + 1 < line_end { p += 2; }
                if p > s + 4 { apply_tag(buffer, "md-strikethrough", s as i32, p as i32); }
                continue;
            }

            p += 1;
        }
    }
}


// ── tag application helper ────────────────────────────────────────────────────
fn apply_tag(buffer: &TextBuffer, name: &str, start_char: i32, end_char: i32) {
    if start_char >= end_char { return; }
    if let Some(tag) = buffer.tag_table().lookup(name) {
        let s = buffer.iter_at_offset(start_char);
        let e = buffer.iter_at_offset(end_char);
        buffer.apply_tag(&tag, &s, &e);
    }
}


// ── code block tokeniser ──────────────────────────────────────────────────────
fn tokenise_block(buffer: &TextBuffer, base: i32, content: &str, lang: &str) {
    let kws        = keywords_for(lang);
    let hash_cmts  = is_hash_comment_lang(lang);
    let rust_lang  = is_rust_lang(lang);
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    macro_rules! tag {
        ($name:expr, $s:expr, $e:expr) => {
            apply_tag(buffer, $name, base + $s as i32, base + $e as i32)
        };
    }

    while pos < len {
        let c = chars[pos];

        // ── block comment /* ... */ ──────────────────────────────────────────
        if c == '/' && pos + 1 < len && chars[pos+1] == '*' {
            let s = pos; pos += 2;
            while pos + 1 < len {
                if chars[pos] == '*' && chars[pos+1] == '/' { pos += 2; break; }
                pos += 1;
            }
            tag!("syn-comment", s, pos);
            continue;
        }

        // ── line comment // ... ──────────────────────────────────────────────
        if c == '/' && pos + 1 < len && chars[pos+1] == '/' {
            let s = pos;
            while pos < len && chars[pos] != '\n' { pos += 1; }
            tag!("syn-comment", s, pos);
            continue;
        }

        // ── Rust inner attribute #! or attribute #[ ──────────────────────────
        if rust_lang && c == '#' && pos + 1 < len &&
           (chars[pos+1] == '[' || (chars[pos+1] == '!' && pos + 2 < len && chars[pos+2] == '['))
        {
            let s = pos;
            while pos < len && chars[pos] != ']' { pos += 1; }
            if pos < len { pos += 1; }
            tag!("syn-decorator", s, pos);
            continue;
        }

        // ── hash comment # ... (Python, shell, etc.) ────────────────────────
        if hash_cmts && c == '#' {
            let s = pos;
            while pos < len && chars[pos] != '\n' { pos += 1; }
            tag!("syn-comment", s, pos);
            continue;
        }

        // ── decorator @ ... ──────────────────────────────────────────────────
        if c == '@' {
            let s = pos; pos += 1;
            while pos < len && (chars[pos].is_alphanumeric() || chars[pos] == '_') { pos += 1; }
            tag!("syn-decorator", s, pos);
            continue;
        }

        // ── raw string r"..." (Rust) ─────────────────────────────────────────
        if c == 'r' && pos + 1 < len && chars[pos+1] == '"' && rust_lang {
            let s = pos; pos += 2;
            while pos < len { if chars[pos] == '"' { pos += 1; break; } pos += 1; }
            tag!("syn-string", s, pos);
            continue;
        }

        // ── double-quoted string ─────────────────────────────────────────────
        if c == '"' {
            let s = pos; pos += 1;
            while pos < len {
                if chars[pos] == '\\' { pos += 2; continue; }
                if chars[pos] == '"'  { pos += 1; break; }
                pos += 1;
            }
            tag!("syn-string", s, pos);
            continue;
        }

        // ── single-quoted literal ────────────────────────────────────────────
        // In Rust, `'ident` is a lifetime, not a string.  Greedily consuming
        // until the next `'` colours entire spans of code as syn-string.
        // Rule: treat as a char literal only for `'x'` (3-char form) or `'\...'`
        // (escape form).  Everything else in Rust is a lifetime → skip the `'`.
        // Non-Rust languages use the original greedy-scan for multi-char strings.
        if c == '\'' {
            let s = pos;
            if rust_lang {
                if pos + 1 < len && chars[pos + 1] == '\\' {
                    // Escaped char literal  '\n', '\t', '\u{...}', etc.
                    pos += 1;
                    while pos < len {
                        if chars[pos] == '\\' { pos += 2; continue; }
                        if chars[pos] == '\'' { pos += 1; break; }
                        pos += 1;
                    }
                    tag!("syn-string", s, pos);
                } else if pos + 2 < len && chars[pos + 2] == '\'' {
                    // Simple char literal  'x'
                    tag!("syn-string", s, s + 3);
                    pos = s + 3;
                } else {
                    // Lifetime  'a, 'static — skip the apostrophe only.
                    pos += 1;
                }
            } else {
                // Python, JS/TS, shell, etc. — greedy multi-char string.
                pos += 1;
                while pos < len {
                    if chars[pos] == '\\' { pos += 2; continue; }
                    if chars[pos] == '\'' { pos += 1; break; }
                    pos += 1;
                }
                tag!("syn-string", s, pos);
            }
            continue;
        }

        // ── backtick template literal (JS/TS) ────────────────────────────────
        if c == '`' {
            let s = pos; pos += 1;
            while pos < len {
                if chars[pos] == '\\' { pos += 2; continue; }
                if chars[pos] == '`'  { pos += 1; break; }
                pos += 1;
            }
            tag!("syn-string", s, pos);
            continue;
        }

        // ── number literal ───────────────────────────────────────────────────
        if c.is_ascii_digit() || (c == '.' && pos + 1 < len && chars[pos+1].is_ascii_digit()) {
            let s = pos;
            while pos < len && (chars[pos].is_ascii_alphanumeric() || chars[pos] == '.' || chars[pos] == '_') {
                pos += 1;
            }
            tag!("syn-number", s, pos);
            continue;
        }

        // ── operator characters ──────────────────────────────────────────────
        if "+-*/%=<>!&|^~?:".contains(c) {
            let s = pos; pos += 1;
            while pos < len && "+-*/%=<>!&|^~?:".contains(chars[pos]) { pos += 1; }
            tag!("syn-operator", s, pos);
            continue;
        }

        // ── identifier: keyword / type / function / macro / const ────────────
        if c.is_alphabetic() || c == '_' {
            let s = pos;
            while pos < len && (chars[pos].is_alphanumeric() || chars[pos] == '_') { pos += 1; }
            let word: String = chars[s..pos].iter().collect();

            // macro call: ends with !
            if pos < len && chars[pos] == '!' {
                pos += 1;
                tag!("syn-macro", s, pos);
                continue;
            }

            // followed by ( → function call
            let next_non_space = {
                let mut i = pos;
                while i < len && chars[i] == ' ' { i += 1; }
                if i < len { Some(chars[i]) } else { None }
            };
            if next_non_space == Some('(') {
                tag!("syn-function", s, pos);
                continue;
            }

            // keyword
            if kws.contains(&word.as_str()) {
                tag!("syn-keyword", s, pos);
                continue;
            }

            // ALL_CAPS identifier → constant
            if word.len() >= 2 && word.chars().all(|ch| ch.is_uppercase() || ch == '_' || ch.is_ascii_digit()) {
                tag!("syn-const", s, pos);
                continue;
            }

            // PascalCase → type
            if word.chars().next().map(|ch| ch.is_uppercase()).unwrap_or(false)
                && word.len() > 1
                && !word.chars().all(|ch| ch.is_uppercase() || ch == '_')
            {
                tag!("syn-type", s, pos);
                continue;
            }

            // everything else: no tag
            continue;
        }

        pos += 1;
    }
}
