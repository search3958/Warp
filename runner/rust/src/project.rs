//! components/ の読み取り（no_std・固定長バッファ）。
//!
//! 対応する記述:
//!   config.ini        : name=
//!   app/main.mb5      : board 名、board 直下の alert / text / list / button、
//!                       config:title / config:text、リストの項目 (N: "...")、
//!                       moveboard(名前) によるボタン遷移、.display(hide)、.type(primary)
//!   models/main.yaml  : 命令の参照名に対応する title / text / message
//! 式評価・スクリプト・イベントの実行など、仕様が未確定の部分は実行しない。

pub const MAX_BOARDS: usize = 8;
pub const MAX_COMPS: usize = 40;
pub const MAX_ITEMS: usize = 8;

/// 固定長の文字列バッファ（no_std 向け）。
#[derive(Clone, Copy)]
pub struct Text<const N: usize> {
    b: [u8; N],
    n: usize,
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Text { b: [0; N], n: 0 }
    }

    pub fn set(&mut self, s: &[u8]) {
        let k = if s.len() < N { s.len() } else { N };
        self.b[..k].copy_from_slice(&s[..k]);
        self.n = k;
    }

    pub fn get(&self) -> &[u8] {
        &self.b[..self.n]
    }
}

#[derive(Clone, Copy)]
pub struct Comp {
    pub board: usize,
    pub kind: Text<16>,
    /// 参照名（yaml のキー）
    pub name: Text<48>,
    /// 見出し（alert/text/list の title、button の文言）
    pub label1: Text<96>,
    /// 本文（alert/text/list の message や text）
    pub label2: Text<96>,
    /// moveboard(...) の遷移先 board 名
    pub event: Text<48>,
    /// .display(hide) が指定されている
    pub hidden: bool,
    /// .type(primary) が指定されている
    pub primary: bool,
    pub items: [Text<64>; MAX_ITEMS],
    pub nitems: usize,
}

const EMPTY_COMP: Comp = Comp {
    board: 0,
    kind: Text::new(),
    name: Text::new(),
    label1: Text::new(),
    label2: Text::new(),
    event: Text::new(),
    hidden: false,
    primary: false,
    items: [Text::new(); MAX_ITEMS],
    nitems: 0,
};

pub struct Project {
    pub title: Text<96>,
    pub boards: [Text<48>; MAX_BOARDS],
    pub nboards: usize,
    pub comps: [Comp; MAX_COMPS],
    pub ncomps: usize,
}

impl Project {
    pub const fn new() -> Self {
        Project {
            title: Text::new(),
            boards: [Text::new(); MAX_BOARDS],
            nboards: 0,
            comps: [EMPTY_COMP; MAX_COMPS],
            ncomps: 0,
        }
    }
}

fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\r'
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
}

pub fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && is_space(s[a]) {
        a += 1;
    }
    while b > a && is_space(s[b - 1]) {
        b -= 1;
    }
    &s[a..b]
}

fn strip_cr(l: &[u8]) -> &[u8] {
    if let Some((&b'\r', rest)) = l.split_last() {
        rest
    } else {
        l
    }
}

/// 先頭から識別子文字（英数字、-、_）が続く部分を返す。
fn ident(s: &[u8]) -> &[u8] {
    let mut n = 0;
    while n < s.len() && is_ident(s[n]) {
        n += 1;
    }
    &s[..n]
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

/// 行末の // コメントを除く。
fn strip_comment(t: &[u8]) -> &[u8] {
    match find(t, b"//") {
        Some(p) => trim(&t[..p]),
        None => t,
    }
}

/// "..." で囲まれていればその中身、なければ値そのもの（前後の空白は除く）。
fn quoted_or_raw(v: &[u8]) -> &[u8] {
    let v = trim(v);
    if v.first() == Some(&b'"') {
        let inner = &v[1..];
        match inner.iter().position(|&c| c == b'"') {
            Some(e) => &inner[..e],
            None => inner,
        }
    } else {
        v
    }
}

/// `key=value` 形式の行から value を返す。
fn ini_value<'a>(line: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let rest = trim(line.strip_prefix(key)?);
    rest.strip_prefix(b"=").map(|v| trim(v))
}

pub fn parse_config(p: &mut Project, src: &[u8]) {
    for line in src.split(|&c| c == b'\n') {
        if let Some(v) = ini_value(trim(line), b"name") {
            p.title.set(v);
        }
    }
}

fn is_kind(k: &[u8]) -> bool {
    k == b"alert" || k == b"text" || k == b"list" || k == b"button"
}

/// `kind(name, ...)` 形式の行を分解する。
/// 戻り値は (kind, 参照名, インラインの "..." 文字列)。
fn parse_component(l: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
    let open = l.iter().position(|&c| c == b'(')?;
    let kind = &l[..open];
    if !is_kind(kind) {
        return None;
    }

    let args = trim(&l[open + 1..]);
    let id = ident(args);
    let ends_ok = match args.get(id.len()) {
        None | Some(&b')') | Some(&b',') => true,
        _ => false,
    };
    let name: &[u8] = if !id.is_empty() && ends_ok { id } else { &args[..0] };

    let inline: &[u8] = match l.iter().position(|&c| c == b'"') {
        Some(a) => {
            let rest = &l[a + 1..];
            match rest.iter().position(|&c| c == b'"') {
                Some(e) => &rest[..e],
                None => &rest[..0],
            }
        }
        None => &l[..0],
    };

    Some((kind, name, inline))
}

/// `moveboard(名前)` の名前を返す。
fn moveboard_target(t: &[u8]) -> &[u8] {
    match find(t, b"moveboard(") {
        Some(p) => ident(&t[p + 10..]),
        None => &t[..0],
    }
}

pub fn parse_mb5(p: &mut Project, src: &[u8]) {
    let mut cur: Option<usize> = None; // 今いる board
    let mut depth: i32 = 0; // 括弧の深さ（複数行の命令の中かどうか）
    let mut last: Option<usize> = None; // 直前に作成した component

    for line in src.split(|&c| c == b'\n') {
        let raw = strip_cr(line);

        // 列0の "}" は board の終わり
        if raw.first() == Some(&b'}') {
            cur = None;
            continue;
        }
        // 列0の "board 名{" は board の始まり
        if raw.starts_with(b"board ") {
            cur = if p.nboards < MAX_BOARDS {
                p.boards[p.nboards].set(ident(&raw[6..]));
                p.nboards += 1;
                Some(p.nboards - 1)
            } else {
                None
            };
            depth = 0;
            last = None;
            continue;
        }
        let board = match cur {
            Some(b) => b,
            None => continue,
        };

        let t = strip_comment(trim(raw));
        if let Some((kind, name, inline)) = parse_component(t) {
            if p.ncomps < MAX_COMPS {
                let idx = p.ncomps;
                let c = &mut p.comps[idx];
                c.board = board;
                c.kind.set(kind);
                c.name.set(name);
                c.label1.set(inline);
                c.event.set(moveboard_target(t));
                p.ncomps += 1;
                last = Some(idx);
            }
        } else if let Some(idx) = last {
            let c = &mut p.comps[idx];
            if t.starts_with(b".type(") {
                c.primary = find(t, b"primary").is_some();
            } else if t.starts_with(b".display(") {
                c.hidden = find(t, b"hide").is_some();
            } else if find(t, b"moveboard(").is_some() {
                c.event.set(moveboard_target(t));
            } else if depth > 0 && t.starts_with(b"config:title:") {
                c.label1.set(quoted_or_raw(&t[13..]));
            } else if depth > 0 && t.starts_with(b"config:text:") {
                let v = quoted_or_raw(&t[12..]);
                if c.label1.get().is_empty() {
                    c.label1.set(v);
                } else {
                    c.label2.set(v);
                }
            } else if t.first().map_or(false, |ch| ch.is_ascii_digit()) && c.kind.get() == b"list" {
                if let Some(colon) = t.iter().position(|&ch| ch == b':') {
                    if c.nitems < MAX_ITEMS {
                        c.items[c.nitems].set(quoted_or_raw(&t[colon + 1..]));
                        c.nitems += 1;
                    }
                }
            }
        }

        // 括弧の深さを更新（コメントは除いて数える）
        for &ch in t {
            if ch == b'(' {
                depth += 1;
            } else if ch == b')' {
                depth -= 1;
            }
        }
        if depth < 0 {
            depth = 0;
        }
    }
}

/// YAML の 1 行から title / text / message の値を取り出す（引用符があれば外す）。
fn yaml_scalar(l: &[u8]) -> Option<&[u8]> {
    let rest = if let Some(r) = l.strip_prefix(b"title:") {
        r
    } else if let Some(r) = l.strip_prefix(b"text:") {
        r
    } else if let Some(r) = l.strip_prefix(b"message:") {
        r
    } else {
        return None;
    };
    Some(quoted_or_raw(rest))
}

/// トップレベルの `key:` ブロック内から、最初の2つの値を返す。
fn yaml_values<'a>(src: &'a [u8], key: &[u8]) -> ([&'a [u8]; 2], usize) {
    let mut vals: [&[u8]; 2] = [&[], &[]];
    let mut n = 0;
    let mut inside = false;
    for raw in src.split(|&c| c == b'\n') {
        let raw = strip_cr(raw);
        if !inside {
            if raw.len() > key.len() && raw.starts_with(key) && &raw[key.len()..] == b":" {
                inside = true;
            }
            continue;
        }
        if raw.is_empty() {
            continue;
        }
        if !is_space(raw[0]) {
            break; // 次のトップレベルのキーに到達
        }
        if let Some(v) = yaml_scalar(trim(raw)) {
            vals[n] = v;
            n += 1;
            if n == 2 {
                break;
            }
        }
    }
    (vals, n)
}

pub fn resolve_labels(p: &mut Project, src: &[u8]) {
    for i in 0..p.ncomps {
        let key = p.comps[i].name;
        if key.get().is_empty() {
            continue;
        }
        let (vals, n) = yaml_values(src, key.get());
        if n >= 1 {
            p.comps[i].label1.set(vals[0]);
        }
        if n >= 2 {
            p.comps[i].label2.set(vals[1]);
        }
    }
}
