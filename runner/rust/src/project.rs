//! MB5 project reader and parser.
//!
//! This module intentionally keeps the runner dependency-free and `no_std`.
//! It parses the structural MB5 subset used by the preview runner: boards,
//! nested component calls, card blocks, chained modifiers, inline model lists,
//! and the corresponding YAML model data. Script execution is not implemented.

pub const MAX_BOARDS: usize = 16;
pub const MAX_COMPS: usize = 128;
pub const MAX_ITEMS: usize = 32;
pub const MAX_MODELS: usize = 64;
const NO_PARENT: usize = usize::MAX;
const MAX_NESTING: usize = 32;

/// Fixed-capacity UTF-8 byte string for the `no_std` runner.
#[derive(Clone, Copy)]
pub struct Text<const N: usize> {
    b: [u8; N],
    n: usize,
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Self { b: [0; N], n: 0 }
    }

    pub fn set(&mut self, value: &[u8]) {
        let mut n = value.len().min(N);
        // Fixed-capacity storage must never cut a UTF-8 code point in half.
        while n > 0 && n < value.len() && (value[n] & 0b1100_0000) == 0b1000_0000 {
            n -= 1;
        }
        self.b[..n].copy_from_slice(&value[..n]);
        self.n = n;
    }

    pub fn get(&self) -> &[u8] {
        &self.b[..self.n]
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

#[derive(Clone, Copy)]
pub struct Comp {
    pub board: usize,
    pub parent: usize,
    pub kind: Text<16>,
    /// YAML model key, e.g. `welcome-text`.
    pub name: Text<64>,
    /// Explicit component identifier supplied by `.id(...)`.
    pub id: Text<64>,
    /// Requested visual size supplied by `.size(...)`.
    pub size: Text<16>,
    pub label1: Text<128>,
    pub label2: Text<192>,
    pub close_label: Text<48>,
    pub config_target: Text<16>,
    pub has_target: bool,
    /// Destination board used by `moveboard(...)`.
    pub event: Text<48>,
    pub hidden: bool,
    pub primary: bool,
    pub radius: i16,
    pub width_px: u16,
    pub width_percent: bool,
    pub bg_color: [u8; 3],
    pub fg_color: [u8; 3],
    pub has_bg_color: bool,
    pub has_fg_color: bool,
    pub has_label1: bool,
    pub has_label2: bool,
    pub items: [Text<96>; MAX_ITEMS],
    pub nitems: usize,
}

const EMPTY_COMP: Comp = Comp {
    board: 0,
    parent: NO_PARENT,
    kind: Text::new(),
    name: Text::new(),
    id: Text::new(),
    size: Text::new(),
    label1: Text::new(),
    label2: Text::new(),
    close_label: Text::new(),
    config_target: Text::new(),
    has_target: false,
    event: Text::new(),
    hidden: false,
    primary: false,
    radius: 12,
    width_px: 0,
    width_percent: false,
    bg_color: [0, 112, 240],
    fg_color: [255, 255, 255],
    has_bg_color: false,
    has_fg_color: false,
    has_label1: false,
    has_label2: false,
    items: [Text::new(); MAX_ITEMS],
    nitems: 0,
};

pub struct Project {
    pub title: Text<96>,
    pub boards: [Text<48>; MAX_BOARDS],
    pub nboards: usize,
    pub main_board: usize,
    pub comps: [Comp; MAX_COMPS],
    pub ncomps: usize,
}

impl Project {
    pub const fn new() -> Self {
        Self {
            title: Text::new(),
            boards: [Text::new(); MAX_BOARDS],
            nboards: 0,
            main_board: 0,
            comps: [EMPTY_COMP; MAX_COMPS],
            ncomps: 0,
        }
    }

    fn add_comp(&mut self, board: usize, parent: usize, kind: &[u8]) -> Option<usize> {
        if self.ncomps >= MAX_COMPS {
            return None;
        }
        let index = self.ncomps;
        let mut comp = EMPTY_COMP;
        comp.board = board;
        comp.parent = parent;
        comp.kind.set(kind);
        self.comps[index] = comp;
        self.ncomps += 1;
        Some(index)
    }
}

fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\r'
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

pub fn trim(s: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = s.len();
    while start < end && is_space(s[start]) {
        start += 1;
    }
    while end > start && is_space(s[end - 1]) {
        end -= 1;
    }
    &s[start..end]
}

fn strip_cr(line: &[u8]) -> &[u8] {
    if line.last() == Some(&b'\r') {
        &line[..line.len() - 1]
    } else {
        line
    }
}

fn strip_ini_comment(line: &[u8]) -> &[u8] {
    let mut quoted = false;
    let mut escaped = false;
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        if c == b'"' && !escaped {
            quoted = !quoted;
        }
        if !quoted && c == b'/' && line.get(i + 1) == Some(&b'/') {
            return trim(&line[..i]);
        }
        escaped = c == b'\\' && !escaped;
        if c != b'\\' {
            escaped = false;
        }
        i += 1;
    }
    trim(line)
}

fn unquote(value: &[u8]) -> &[u8] {
    let v = trim(value);
    if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        &v[1..v.len() - 1]
    } else {
        v
    }
}

fn ini_value<'a>(line: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let rest = trim(line.strip_prefix(key)?);
    rest.strip_prefix(b"=").map(trim)
}

pub fn parse_config(project: &mut Project, src: &[u8]) {
    for raw in src.split(|&c| c == b'\n') {
        let line = strip_ini_comment(strip_cr(raw));
        if let Some(value) = ini_value(line, b"name") {
            project.title.set(unquote(value));
        }
    }
}

fn is_component_kind(name: &[u8]) -> bool {
    name == b"alert" || name == b"text" || name == b"list" || name == b"button"
}

struct Mb5Parser<'a, 'p> {
    src: &'a [u8],
    pos: usize,
    line: usize,
    project: &'p mut Project,
    errors: usize,
    main_board_pending: bool,
}

impl<'a, 'p> Mb5Parser<'a, 'p> {
    fn new(project: &'p mut Project, src: &'a [u8]) -> Self {
        Self { src, pos: 0, line: 1, project, errors: 0, main_board_pending: false }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_n(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn error(&mut self, message: &[u8]) {
        self.errors += 1;
        crate::sys::eprint(b"main.mb5:");
        crate::sys::write_num(crate::sys::STDERR, self.line);
        crate::sys::eprint(b": ");
        crate::sys::eprint(message);
        crate::sys::eprint(b"\n");
    }

    fn skip_horizontal(&mut self) {
        while self.peek().map_or(false, is_space) {
            self.bump();
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some(b'/') && self.peek_n(1) == Some(b'/') {
            while let Some(c) = self.bump() {
                if c == b'\n' {
                    break;
                }
            }
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            self.skip_horizontal();
            if self.peek() == Some(b'/') && self.peek_n(1) == Some(b'/') {
                self.skip_comment();
                continue;
            }
            if self.peek() == Some(b'\n') {
                self.bump();
                continue;
            }
            break;
        }
    }

    fn read_ident(&mut self) -> &'a [u8] {
        let start = self.pos;
        while self.peek().map_or(false, is_ident) {
            self.bump();
        }
        &self.src[start..self.pos]
    }

    fn read_quoted(&mut self) -> Option<&'a [u8]> {
        if self.peek() != Some(b'"') {
            return None;
        }
        self.bump();
        let start = self.pos;
        let mut escaped = false;
        let mut crossed_line = false;
        while let Some(c) = self.bump() {
            if c == b'"' && !escaped {
                if crossed_line {
                    self.error("文字列は1行で記述してください".as_bytes());
                }
                return Some(&self.src[start..self.pos - 1]);
            }
            if c == b'\n' {
                crossed_line = true;
            }
            if c == b'\\' && !escaped {
                escaped = true;
            } else {
                escaped = false;
            }
        }
        self.error("閉じられていない文字列です".as_bytes());
        Some(&self.src[start..self.pos])
    }

    /// Reads a single scalar. Unquoted values end at a comma, line break,
    /// closing parenthesis/bracket/brace, or comment.
    fn read_scalar(&mut self) -> &'a [u8] {
        self.skip_horizontal();
        if self.peek() == Some(b'"') {
            return self.read_quoted().unwrap_or(&self.src[self.pos..self.pos]);
        }
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == b'\n' || c == b',' || c == b')' || c == b']' || c == b'}' {
                break;
            }
            if c == b'/' && self.peek_n(1) == Some(b'/') {
                break;
            }
            self.bump();
        }
        trim(&self.src[start..self.pos])
    }

    fn consume(&mut self, expected: u8) -> bool {
        self.skip_horizontal();
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_line(&mut self) {
        while let Some(c) = self.bump() {
            if c == b'\n' {
                break;
            }
        }
    }

    fn skip_balanced_block(&mut self) {
        if !self.consume(b'{') {
            self.skip_line();
            return;
        }
        self.skip_block_after_open();
    }

    fn skip_block_after_open(&mut self) {
        let mut depth = 1usize;
        let mut in_string = false;
        let mut escaped = false;
        while let Some(c) = self.bump() {
            if in_string {
                if c == b'"' && !escaped {
                    in_string = false;
                }
                if c == b'\\' && !escaped {
                    escaped = true;
                } else {
                    escaped = false;
                }
                continue;
            }
            if c == b'"' {
                in_string = true;
            } else if c == b'/' && self.peek() == Some(b'/') {
                self.bump();
                while let Some(comment_char) = self.bump() {
                    if comment_char == b'\n' { break; }
                }
            } else if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
        }
        self.error("閉じられていないブロックです".as_bytes());
    }

    fn parse(mut self) -> usize {
        while self.pos < self.src.len() {
            self.skip_trivia();
            if self.pos >= self.src.len() {
                break;
            }
            if self.peek() == Some(b'@') {
                self.bump();
                let directive = self.read_ident();
                self.main_board_pending = directive == b"mainBoard";
                self.skip_line();
                continue;
            }

            let saved = self.pos;
            let name = self.read_ident();
            if name == b"board" {
                self.skip_horizontal();
                let board_name = self.read_ident();
                self.skip_horizontal();
                if board_name.is_empty() || !self.consume(b'{') {
                    self.error("board 定義は `board name { ... }` の形式で記述してください".as_bytes());
                    self.skip_line();
                    continue;
                }
                if self.project.nboards >= MAX_BOARDS {
                    self.error("board の上限を超えました".as_bytes());
                    self.skip_block_after_open();
                    self.main_board_pending = false;
                    continue;
                }
                let board = self.project.nboards;
                self.project.boards[board].set(board_name);
                self.project.nboards += 1;
                if self.project.nboards == 1 || self.main_board_pending {
                    self.project.main_board = board;
                }
                self.main_board_pending = false;
                self.parse_body(board, NO_PARENT, 0);
                continue;
            }

            // Ignore declarations outside boards (structs and scripts are not
            // executed by this preview runner, but their braces are balanced).
            self.pos = saved;
            self.skip_unknown_top_level();
        }
        self.errors
    }

    fn skip_unknown_top_level(&mut self) {
        let mut in_string = false;
        let mut escaped = false;
        while let Some(c) = self.peek() {
            if in_string {
                self.bump();
                if c == b'"' && !escaped {
                    in_string = false;
                }
                if c == b'\\' && !escaped {
                    escaped = true;
                } else {
                    escaped = false;
                }
                continue;
            }
            if c == b'"' {
                in_string = true;
                self.bump();
            } else if c == b'/' && self.peek_n(1) == Some(b'/') {
                self.skip_comment();
                if self.peek().is_none() {
                    return;
                }
            } else if c == b'{' {
                self.skip_balanced_block();
                return;
            } else if c == b'\n' {
                self.bump();
                return;
            } else {
                self.bump();
            }
        }
    }

    fn parse_body(&mut self, board: usize, parent: usize, nesting: usize) {
        if nesting > MAX_NESTING {
            self.error("コンポーネントのネストが深すぎます".as_bytes());
            return;
        }
        loop {
            self.skip_trivia();
            match self.peek() {
                None => {
                    self.error("board の閉じ括弧 `}` がありません".as_bytes());
                    return;
                }
                Some(b'}') => {
                    self.bump();
                    return;
                }
                Some(b';') | Some(b',') => {
                    self.bump();
                    continue;
                }
                _ => {}
            }

            let start = self.pos;
            let word = self.read_ident();
            if word.is_empty() {
                self.bump();
                continue;
            }
            self.skip_horizontal();

            if word == b"card" && self.peek() == Some(b'{') {
                self.bump();
                self.parse_body(board, parent, nesting + 1);
                continue;
            }

            if is_component_kind(word) && self.peek() == Some(b'(') {
                if let Some(index) = self.project.add_comp(board, parent, word) {
                    self.bump();
                    self.parse_component_args(index, nesting + 1);
                    self.parse_modifiers(index);
                } else {
                    self.error("コンポーネント数の上限を超えました".as_bytes());
                    self.skip_call();
                }
                continue;
            }

            // Fail visibly rather than silently dropping syntax the preview
            // runner cannot interpret.
            self.error("board 内に未対応の命令があります".as_bytes());
            self.pos = start;
            self.skip_unknown_statement();
        }
    }

    fn skip_unknown_statement(&mut self) {
        let mut parens = 0usize;
        let mut brackets = 0usize;
        let mut braces = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        while let Some(c) = self.peek() {
            if in_string {
                self.bump();
                if c == b'"' && !escaped {
                    in_string = false;
                }
                if c == b'\\' && !escaped {
                    escaped = true;
                } else {
                    escaped = false;
                }
                continue;
            }
            match c {
                b'"' => { in_string = true; self.bump(); }
                b'/' if self.peek_n(1) == Some(b'/') => { self.skip_comment(); return; }
                b'(' => { parens += 1; self.bump(); }
                b')' => {
                    if parens == 0 { self.bump(); return; }
                    parens -= 1; self.bump();
                }
                b'[' => { brackets += 1; self.bump(); }
                b']' => { if brackets > 0 { brackets -= 1; } self.bump(); }
                b'{' => { braces += 1; self.bump(); }
                b'}' => {
                    if braces == 0 && parens == 0 && brackets == 0 { return; }
                    if braces > 0 { braces -= 1; }
                    self.bump();
                }
                b'\n' if parens == 0 && brackets == 0 && braces == 0 => { self.bump(); return; }
                _ => { self.bump(); }
            }
        }
    }

    fn skip_call(&mut self) {
        if !self.consume(b'(') { return; }
        let mut depth = 1usize;
        while let Some(c) = self.bump() {
            if c == b'(' { depth += 1; }
            if c == b')' {
                depth -= 1;
                if depth == 0 { return; }
            }
        }
    }

    fn parse_component_args(&mut self, index: usize, nesting: usize) {
        if nesting > MAX_NESTING {
            self.error("コンポーネントのネストが深すぎます".as_bytes());
            self.skip_call();
            return;
        }
        loop {
            self.skip_trivia();
            match self.peek() {
                None => {
                    self.error("コンポーネントの `)` がありません".as_bytes());
                    return;
                }
                Some(b')') => { self.bump(); return; }
                Some(b',') => { self.bump(); continue; }
                Some(b'[') => {
                    self.parse_inline_model(index);
                    continue;
                }
                Some(b'(') => {
                    self.error("引数の位置に予期しない `(` があります".as_bytes());
                    self.bump();
                    continue;
                }
                _ => {}
            }

            let key = self.read_ident();
            if key.is_empty() {
                self.bump();
                continue;
            }
            self.skip_horizontal();

            if self.peek() == Some(b':') {
                self.bump();
                if key == b"config" {
                    self.parse_config_property(index);
                } else if key == b"event" {
                    self.parse_event_property(index);
                } else if key == b"events" {
                    // Event maps are YAML-side definitions; keep the syntax
                    // balanced here without pretending to execute them.
                    self.skip_property_value();
                } else if key == b"model" {
                    self.skip_horizontal();
                    if self.peek() == Some(b'[') {
                        self.parse_inline_model(index);
                    } else if self.peek() == Some(b'\n') || self.peek() == Some(b')') || self.peek().is_none() {
                        self.error("model に値がありません。複数行データは `[...]` で囲んでください".as_bytes());
                    } else {
                        // An unbracketed model is deliberately limited to one
                        // physical line. A one-line scalar or key:value pair is
                        // accepted; multiline input is rejected at `model:`.
                        let raw = self.read_scalar();
                        let value = match raw.iter().position(|&c| c == b':') {
                            Some(colon) => unquote(trim(&raw[colon + 1..])),
                            None => unquote(raw),
                        };
                        if value.is_empty() {
                            self.error("model の値が空です".as_bytes());
                        } else if self.project.comps[index].nitems < MAX_ITEMS {
                            let comp = &mut self.project.comps[index];
                            comp.items[comp.nitems].set(value);
                            comp.nitems += 1;
                        }
                    }
                } else {
                    self.error("未対応の引数キーです".as_bytes());
                    self.skip_property_value();
                }
                continue;
            }

            if self.peek() == Some(b'(') {
                if is_component_kind(key) {
                    let board = self.project.comps[index].board;
                    if let Some(child) = self.project.add_comp(board, index, key) {
                        self.bump();
                        self.parse_component_args(child, nesting + 1);
                        self.parse_modifiers(child);
                    } else {
                        self.error("コンポーネント数の上限を超えました".as_bytes());
                        self.skip_call();
                    }
                } else if key == b"moveboard" {
                    self.bump();
                    self.parse_moveboard_call(index);
                } else {
                    self.error("引数内に未対応の関数呼び出しがあります".as_bytes());
                    self.skip_call();
                }
                continue;
            }

            // The first bare identifier is the model reference. Additional
            // bare tokens are invalid rather than being silently reinterpreted.
            if self.project.comps[index].name.is_empty() {
                self.project.comps[index].name.set(key);
            } else {
                self.error("モデル参照以外の位置に裸の識別子があります".as_bytes());
            }
        }
    }

    fn parse_config_property(&mut self, index: usize) {
        self.skip_horizontal();
        let property = self.read_ident();
        if property.is_empty() || !self.consume(b':') {
            self.error("config は `config:property: value` の形式で記述してください".as_bytes());
            self.skip_property_value();
            return;
        }
        let value = self.read_scalar();
        let mut invalid_config = false;
        {
            let comp = &mut self.project.comps[index];
            if property == b"title" {
                comp.label1.set(value);
                comp.has_label1 = true;
            } else if property == b"text" || property == b"message" {
                if comp.kind.get() == b"button" {
                    comp.label1.set(value);
                    comp.has_label1 = true;
                } else {
                    comp.label2.set(value);
                    comp.has_label2 = true;
                }
            } else if property == b"target" {
                comp.config_target.set(value);
                comp.has_target = true;
                invalid_config = value != comp.kind.get();
            } else {
                invalid_config = true;
            }
        }
        if invalid_config {
            self.error("config のキーが未対応、または target とコンポーネントの種類が一致しません".as_bytes());
        }
    }

    fn parse_event_property(&mut self, index: usize) {
        self.skip_trivia();
        let key = self.read_ident();
        self.skip_horizontal();
        if key.is_empty() {
            self.error("event の値がありません".as_bytes());
            self.skip_property_value();
            return;
        }
        if key == b"moveboard" && self.peek() == Some(b'(') {
            self.bump();
            self.skip_trivia();
            let target = self.read_ident();
            if target.is_empty() {
                self.error("moveboard の遷移先がありません".as_bytes());
            }
            self.project.comps[index].event.set(target);
            self.skip_horizontal();
            if !self.consume(b')') {
                self.error("moveboard の `)` がありません".as_bytes());
                self.skip_property_value();
            }
        } else {
            self.skip_property_value();
        }
    }

    fn parse_moveboard_call(&mut self, index: usize) {
        self.skip_trivia();
        let target = self.read_ident();
        if target.is_empty() {
            self.error("moveboard の遷移先がありません".as_bytes());
        }
        self.project.comps[index].event.set(target);
        self.skip_horizontal();
        if !self.consume(b')') {
            self.error("moveboard の `)` がありません".as_bytes());
            self.skip_property_value();
        }
    }

    fn skip_property_value(&mut self) {
        self.skip_horizontal();
        if self.peek() == Some(b'"') {
            let _ = self.read_quoted();
            return;
        }
        let mut parens = 0usize;
        while let Some(c) = self.peek() {
            if c == b'\n' || c == b',' || (c == b')' && parens == 0) || c == b']' {
                return;
            }
            if c == b'(' { parens += 1; }
            if c == b')' && parens > 0 { parens -= 1; }
            if c == b'/' && self.peek_n(1) == Some(b'/') { return; }
            self.bump();
        }
    }

    fn parse_inline_model(&mut self, index: usize) {
        if !self.consume(b'[') {
            self.error("インラインモデルの `[` がありません".as_bytes());
            return;
        }
        let start_count = self.project.comps[index].nitems;
        loop {
            self.skip_trivia();
            match self.peek() {
                None => {
                    self.error("インラインモデルの `]` がありません".as_bytes());
                    return;
                }
                Some(b']') => {
                    self.bump();
                    if self.project.comps[index].nitems == start_count {
                        self.error("インラインモデルに項目がありません".as_bytes());
                    }
                    return;
                }
                Some(b',') => { self.bump(); continue; }
                _ => {}
            }
            let key_start = self.pos;
            while self.peek().map_or(false, |c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-') {
                self.bump();
            }
            let key_end = self.pos;
            if self.pos == key_start || !self.consume(b':') {
                self.error("インラインモデルの項目は `key: value` の形式で記述してください".as_bytes());
                self.skip_line();
                continue;
            }
            let value = self.read_scalar();
            if key_is_model_header(&self.src[key_start..key_end], value) {
                // `[` may contain a YAML-style `model:` section header whose
                // entries continue on subsequent lines.
                continue;
            }
            if value.is_empty() {
                self.error("インラインモデルの項目に値がありません".as_bytes());
            } else if self.project.comps[index].nitems < MAX_ITEMS {
                let comp = &mut self.project.comps[index];
                comp.items[comp.nitems].set(value);
                comp.nitems += 1;
            } else {
                self.error("モデル項目数の上限を超えました".as_bytes());
            }
        }
    }

    fn parse_modifiers(&mut self, index: usize) {
        loop {
            let saved = self.pos;
            let saved_line = self.line;
            self.skip_trivia();
            if self.peek() != Some(b'.') {
                self.pos = saved;
                self.line = saved_line;
                return;
            }
            self.bump();
            let method = self.read_ident();
            self.skip_horizontal();
            if method.is_empty() || !self.consume(b'(') {
                self.error("修飾子は `.name(value)` の形式で記述してください".as_bytes());
                self.skip_line();
                return;
            }
            self.skip_horizontal();
            let value = if self.peek() == Some(b'"') {
                self.read_quoted().unwrap_or(&self.src[self.pos..self.pos])
            } else {
                self.read_scalar()
            };
            self.skip_horizontal();
            if !self.consume(b')') {
                self.error("修飾子の `)` がありません".as_bytes());
                self.skip_line();
                return;
            }
            let mut invalid_modifier = false;
            {
                let comp = &mut self.project.comps[index];
                if method == b"id" {
                    comp.id.set(value);
                } else if method == b"size" {
                    if value == b"small" || value == b"medium" || value == b"large" {
                        comp.size.set(value);
                    } else {
                        invalid_modifier = true;
                    }
                } else if method == b"display" {
                    if value == b"hide" {
                        comp.hidden = true;
                    } else if value == b"show" {
                        comp.hidden = false;
                    } else {
                        invalid_modifier = true;
                    }
                } else if method == b"type" {
                    if value == b"primary" {
                        comp.primary = true;
                    } else if value == b"text" {
                        comp.primary = false;
                    } else {
                        invalid_modifier = true;
                    }
                } else if method == b"radius" {
                    if let Some(n) = parse_u16(value) {
                        comp.radius = n.min(i16::MAX as u16) as i16;
                    } else {
                        invalid_modifier = true;
                    }
                } else if method == b"width" {
                    if let Some((n, percent)) = parse_width(value) {
                        comp.width_px = n;
                        comp.width_percent = percent;
                    } else {
                        invalid_modifier = true;
                    }
                } else if method == b"bgColor" {
                    if let Some(color) = parse_hex_color(value) {
                        comp.bg_color = color;
                        comp.has_bg_color = true;
                    } else {
                        invalid_modifier = true;
                    }
                } else if method == b"fgColor" {
                    if let Some(color) = parse_hex_color(value) {
                        comp.fg_color = color;
                        comp.has_fg_color = true;
                    } else {
                        invalid_modifier = true;
                    }
                }
            }
            if invalid_modifier {
                self.error("修飾子の値が不正です".as_bytes());
            }
            // Future modifiers are still consumed as complete chain elements
            // so they cannot desynchronize parsing of the following component.
        }
    }
}

fn key_is_model_header(key_slice: &[u8], value: &[u8]) -> bool {
    trim(key_slice) == b"model" && value.is_empty()
}

fn parse_u16(value: &[u8]) -> Option<u16> {
    let mut n = 0u16;
    let mut any = false;
    for &c in value {
        if c.is_ascii_digit() {
            any = true;
            n = n.checked_mul(10)?.checked_add((c - b'0') as u16)?;
        } else if c == b'p' || c == b'x' || c == b'%' {
            break;
        } else if !is_space(c) {
            return None;
        }
    }
    if any { Some(n) } else { None }
}

fn parse_width(value: &[u8]) -> Option<(u16, bool)> {
    let v = trim(value);
    let percent = v.ends_with(b"%");
    let numeric = if percent { &v[..v.len() - 1] } else if v.ends_with(b"px") {
        &v[..v.len() - 2]
    } else {
        v
    };
    let n = parse_u16(numeric)?;
    if percent && n > 100 { return None; }
    Some((n, percent))
}

fn parse_hex_color(value: &[u8]) -> Option<[u8; 3]> {
    let v = trim(value);
    if v.len() != 7 || v[0] != b'#' {
        return None;
    }
    let mut out = [0u8; 3];
    for i in 0..3 {
        let a = hex_digit(v[1 + i * 2])?;
        let b = hex_digit(v[2 + i * 2])?;
        out[i] = a * 16 + b;
    }
    Some(out)
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

pub fn parse_mb5(project: &mut Project, src: &[u8]) -> usize {
    Mb5Parser::new(project, src).parse()
}

/// Validate references that can be checked without executing scripts.
pub fn validate_structure(project: &Project) -> usize {
    let mut errors = 0usize;
    if project.nboards == 0 {
        crate::sys::eprint("main.mb5: board が定義されていません\n".as_bytes());
        errors += 1;
    }
    for i in 0..project.nboards {
        for j in i + 1..project.nboards {
            if project.boards[i].get() == project.boards[j].get() {
                crate::sys::eprint(b"main.mb5: duplicate board name: ");
                crate::sys::eprint(project.boards[i].get());
                crate::sys::eprint(b"\n");
                errors += 1;
            }
        }
    }
    for i in 0..project.ncomps {
        let target = project.comps[i].event.get();
        if target.is_empty() { continue; }
        let found = (0..project.nboards).any(|b| project.boards[b].get() == target);
        if !found {
            crate::sys::eprint(b"main.mb5: moveboard target not found: ");
            crate::sys::eprint(target);
            crate::sys::eprint(b"\n");
            errors += 1;
        }
    }
    errors
}

#[derive(Clone, Copy)]
struct ModelItem {
    key: Text<48>,
    value: Text<128>,
}

const EMPTY_MODEL_ITEM: ModelItem = ModelItem { key: Text::new(), value: Text::new() };

#[derive(Clone, Copy)]
struct Model {
    name: Text<64>,
    target: Text<16>,
    title: Text<128>,
    text: Text<192>,
    message: Text<192>,
    items: [ModelItem; MAX_ITEMS],
    nitems: usize,
}

const EMPTY_MODEL: Model = Model {
    name: Text::new(),
    target: Text::new(),
    title: Text::new(),
    text: Text::new(),
    message: Text::new(),
    items: [EMPTY_MODEL_ITEM; MAX_ITEMS],
    nitems: 0,
};

#[derive(Clone, Copy, PartialEq)]
enum YamlSection {
    None,
    Config,
    Model,
    Event,
    Events,
}

fn yaml_pair(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut quoted = false;
    let mut escaped = false;
    for (i, &c) in line.iter().enumerate() {
        if c == b'"' && !escaped { quoted = !quoted; }
        if c == b':' && !quoted {
            return Some((trim(&line[..i]), trim(&line[i + 1..])));
        }
        if c == b'\\' && !escaped { escaped = true; } else { escaped = false; }
    }
    None
}

fn yaml_scalar(value: &[u8]) -> &[u8] {
    let v = trim(value);
    let mut quoted = false;
    let mut escaped = false;
    let mut end = v.len();
    for (i, &c) in v.iter().enumerate() {
        if c == b'"' && !escaped { quoted = !quoted; }
        if !quoted && c == b'#' && (i == 0 || is_space(v[i - 1])) {
            end = i;
            break;
        }
        if c == b'\\' && !escaped { escaped = true; } else { escaped = false; }
    }
    unquote(trim(&v[..end]))
}

fn yaml_indent(line: &[u8]) -> usize {
    line.iter().take_while(|&&c| c == b' ').count()
}

fn find_model(models: &[Model; MAX_MODELS], count: usize, key: &[u8]) -> Option<usize> {
    (0..count).find(|&i| models[i].name.get() == key)
}

fn append_model_item(model: &mut Model, key: &[u8], value: &[u8]) {
    if model.nitems >= MAX_ITEMS { return; }
    let index = model.nitems;
    model.items[index].key.set(key);
    model.items[index].value.set(value);
    model.nitems += 1;
}

/// Resolve simple, indentation-based YAML used by MB5 models. This is not a
/// general YAML 1.2 parser; unsupported YAML constructs are intentionally not
/// guessed or coerced.
pub fn resolve_labels(project: &mut Project, src: &[u8]) -> usize {
    let mut models = [EMPTY_MODEL; MAX_MODELS];
    let mut nmodels = 0usize;
    let mut section = YamlSection::None;
    let mut current_item: Option<usize> = None;

    for raw in src.split(|&c| c == b'\n') {
        let raw = strip_cr(raw);
        let line = trim(raw);
        if line.is_empty() || line.starts_with(b"#") || line == b"---" {
            continue;
        }
        let indent = yaml_indent(raw);
        let pair = match yaml_pair(line) {
            Some(pair) => pair,
            None => continue,
        };
        let (key, value) = pair;

        if indent == 0 {
            if nmodels >= MAX_MODELS {
                continue;
            }
            models[nmodels] = EMPTY_MODEL;
            models[nmodels].name.set(key);
            nmodels += 1;
            section = YamlSection::None;
            current_item = None;
            continue;
        }

        let model_index = nmodels.saturating_sub(1);
        if nmodels == 0 { continue; }

        if indent == 2 && value.is_empty() {
            section = if key == b"config" {
                YamlSection::Config
            } else if key == b"model" {
                YamlSection::Model
            } else if key == b"event" {
                YamlSection::Event
            } else if key == b"events" {
                YamlSection::Events
            } else {
                YamlSection::None
            };
            current_item = None;
            continue;
        }

        match section {
            YamlSection::Config if indent >= 4 => {
                let value = yaml_scalar(value);
                let model = &mut models[model_index];
                if key == b"target" { model.target.set(value); }
                else if key == b"title" { model.title.set(value); }
                else if key == b"text" { model.text.set(value); }
                else if key == b"message" { model.message.set(value); }
            }
            YamlSection::Model if indent == 4 => {
                let model = &mut models[model_index];
                append_model_item(model, key, yaml_scalar(value));
                current_item = Some(model.nitems - 1);
            }
            YamlSection::Model if indent >= 6 => {
                if let Some(item_index) = current_item {
                    if key == b"text" {
                        models[model_index].items[item_index].value.set(yaml_scalar(value));
                    }
                }
            }
            YamlSection::Event | YamlSection::Events => {}
            _ => {}
        }
    }

    let mut errors = 0usize;
    for i in 0..project.ncomps {
        let mut name_copy = Text::<64>::new();
        name_copy.set(project.comps[i].name.get());
        let name = name_copy.get();
        if name.is_empty() { continue; }
        let model_index = match find_model(&models, nmodels, name) {
            Some(index) => index,
            None => {
                crate::sys::eprint(b"main.yaml: model reference not found: ");
                crate::sys::eprint(name);
                crate::sys::eprint(b"\n");
                errors += 1;
                continue;
            }
        };
        let model = models[model_index];
        let comp = &mut project.comps[i];

        if !model.target.is_empty() && model.target.get() != comp.kind.get() {
            crate::sys::eprint(b"main.yaml: model target does not match component `");
            crate::sys::eprint(comp.kind.get());
            crate::sys::eprint(b"`\n");
            errors += 1;
        }
        if comp.has_target && !model.target.is_empty() {
            crate::sys::eprint(b"main.yaml: YAML config and MB5 config conflict for model `");
            crate::sys::eprint(name);
            crate::sys::eprint(b"` (target)\n");
            errors += 1;
        }
        if comp.has_target && comp.config_target.get() != comp.kind.get() {
            crate::sys::eprint(b"main.mb5: config target does not match component `");
            crate::sys::eprint(comp.kind.get());
            crate::sys::eprint(b"`\n");
            errors += 1;
        }

        let yaml_label1 = if comp.kind.get() == b"button" {
            model.text.get()
        } else {
            model.title.get()
        };
        let yaml_label2: &[u8] = if comp.kind.get() == b"button" {
            &[]
        } else if comp.kind.get() == b"alert" && !model.message.is_empty() {
            model.message.get()
        } else {
            model.text.get()
        };

        if !yaml_label1.is_empty() {
            if comp.has_label1 {
                crate::sys::eprint(b"main.yaml: YAML config and MB5 config conflict for model `");
                crate::sys::eprint(name);
                crate::sys::eprint(b"` (title/text)\n");
                errors += 1;
            } else {
                comp.label1.set(yaml_label1);
            }
        }
        if !yaml_label2.is_empty() {
            if comp.has_label2 {
                crate::sys::eprint(b"main.yaml: YAML config and MB5 config conflict for model `");
                crate::sys::eprint(name);
                crate::sys::eprint(b"` (text/message)\n");
                errors += 1;
            } else {
                comp.label2.set(yaml_label2);
            }
        }

        if comp.kind.get() == b"alert" {
            for item in model.items[..model.nitems].iter() {
                if item.key.get() == b"close" {
                    comp.close_label.set(item.value.get());
                }
            }
        } else if comp.kind.get() == b"list" {
            if comp.nitems > 0 && model.nitems > 0 {
                crate::sys::eprint(b"main.yaml: inline model and YAML model both define list items for `");
                crate::sys::eprint(name);
                crate::sys::eprint(b"`\n");
                errors += 1;
            } else {
                for item in model.items[..model.nitems].iter() {
                    if item.value.is_empty() { continue; }
                    if comp.nitems < MAX_ITEMS {
                        comp.items[comp.nitems].set(item.value.get());
                        comp.nitems += 1;
                    }
                }
            }
        }
    }
    errors
}

