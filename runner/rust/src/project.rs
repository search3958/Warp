//! MB5 project reader.
//! Fixed-capacity data structures are used to keep the runner no_std-friendly.

pub const MAX_BOARDS: usize = 8;
pub const MAX_COMPS: usize = 48;
pub const MAX_ITEMS: usize = 16;
pub const MAX_SCRIPTS: usize = 24;
pub const MAX_COMMANDS: usize = 32;
pub const MAX_STRUCTS: usize = 16;
pub const MAX_STRUCT_COMPS: usize = 8;
pub const NO_PARENT: usize = usize::MAX;

#[derive(Clone, Copy)]
pub struct Text<const N: usize> {
    b: [u8; N],
    n: usize,
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Self { b: [0; N], n: 0 }
    }

    pub fn set(&mut self, s: &[u8]) {
        let mut k = core::cmp::min(s.len(), N);
        // Never leave a partial UTF-8 code point at the end of the fixed buffer.
        while k > 0 && k < s.len() && (s[k] & 0xC0) == 0x80 {
            k -= 1;
        }
        self.b[..k].copy_from_slice(&s[..k]);
        self.n = k;
    }

    pub fn get(&self) -> &[u8] {
        &self.b[..self.n]
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

#[derive(Clone, Copy)]
pub struct ScriptCall {
    pub name: Text<48>,
    pub arg: Text<48>,
}

impl ScriptCall {
    pub const fn new() -> Self {
        Self { name: Text::new(), arg: Text::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

#[derive(Clone, Copy)]
pub struct ModelItem {
    pub id: Text<48>,
    pub text: Text<96>,
    pub call: ScriptCall,
}

const EMPTY_ITEM: ModelItem = ModelItem { id: Text::new(), text: Text::new(), call: ScriptCall::new() };

#[derive(Clone, Copy)]
pub struct Comp {
    pub board: usize,
    pub parent: usize,
    pub kind: Text<16>,
    /// YAML model key referenced by the component.
    pub name: Text<48>,
    /// Parent struct name for dynamically added components.
    pub struct_name: Text<48>,
    /// Explicit `.id(...)`, used by setComponentConfig().
    pub id: Text<48>,
    pub label1: Text<96>,
    pub label2: Text<128>,
    /// moveboard() target.
    pub event: Text<48>,
    pub close_event: ScriptCall,
    pub close_label: Text<48>,
    pub hidden: bool,
    pub primary: bool,
    /// `.type(text)` renders as a transparent text button.
    pub text_button: bool,
    pub deleted: bool,
    pub width: Text<24>,
    pub size: Text<16>,
    pub bg_color: [u8; 3],
    pub fg_color: [u8; 3],
    pub has_bg_color: bool,
    pub has_fg_color: bool,
    pub radius: i32,
    /// True when `.radius(...)` was explicitly supplied.
    pub radius_custom: bool,
    pub items: [ModelItem; MAX_ITEMS],
    pub nitems: usize,
}

const EMPTY_COMP: Comp = Comp {
    board: 0,
    parent: NO_PARENT,
    kind: Text::new(),
    name: Text::new(),
    struct_name: Text::new(),
    id: Text::new(),
    label1: Text::new(),
    label2: Text::new(),
    event: Text::new(),
    close_event: ScriptCall::new(),
    close_label: Text::new(),
    hidden: false,
    primary: false,
    text_button: false,
    deleted: false,
    width: Text::new(),
    size: Text::new(),
    bg_color: [0, 106, 255],
    fg_color: [26, 26, 26],
    has_bg_color: false,
    has_fg_color: false,
    radius: 16,
    radius_custom: false,
    items: [EMPTY_ITEM; MAX_ITEMS],
    nitems: 0,
};

pub const CMD_WAIT: u8 = 1;
pub const CMD_SET_DISPLAY: u8 = 2;
pub const CMD_CALL: u8 = 3;
pub const CMD_ADD_COMPONENT: u8 = 4;
pub const CMD_DELETE_COMPONENT: u8 = 5;

#[derive(Clone, Copy)]
pub struct ScriptCommand {
    pub kind: u8,
    pub subject: Text<48>,
    pub arg: Text<48>,
    pub target: Text<48>,
    pub millis: u64,
    pub flag: bool,
}

const EMPTY_COMMAND: ScriptCommand = ScriptCommand {
    kind: 0,
    subject: Text::new(),
    arg: Text::new(),
    target: Text::new(),
    millis: 0,
    flag: false,
};

#[derive(Clone, Copy)]
pub struct Script {
    pub name: Text<48>,
    /// Empty for a normal script; non-empty for a message(show) branch.
    pub branch: Text<48>,
    pub commands: [ScriptCommand; MAX_COMMANDS],
    pub ncommands: usize,
}

const EMPTY_SCRIPT: Script = Script {
    name: Text::new(),
    branch: Text::new(),
    commands: [EMPTY_COMMAND; MAX_COMMANDS],
    ncommands: 0,
};

#[derive(Clone, Copy)]
pub struct StructDef {
    pub name: Text<48>,
    pub comps: [Comp; MAX_STRUCT_COMPS],
    pub ncomps: usize,
}

const EMPTY_STRUCT: StructDef = StructDef {
    name: Text::new(),
    comps: [EMPTY_COMP; MAX_STRUCT_COMPS],
    ncomps: 0,
};

pub struct Project {
    pub title: Text<96>,
    pub boards: [Text<48>; MAX_BOARDS],
    pub nboards: usize,
    pub main_board: Text<48>,
    pub comps: [Comp; MAX_COMPS],
    pub ncomps: usize,
    pub scripts: [Script; MAX_SCRIPTS],
    pub nscripts: usize,
    pub structs: [StructDef; MAX_STRUCTS],
    pub nstructs: usize,
    pub main_script: Text<48>,
}

impl Project {
    pub const fn new() -> Self {
        Self {
            title: Text::new(),
            boards: [Text::new(); MAX_BOARDS],
            nboards: 0,
            main_board: Text::new(),
            comps: [EMPTY_COMP; MAX_COMPS],
            ncomps: 0,
            scripts: [EMPTY_SCRIPT; MAX_SCRIPTS],
            nscripts: 0,
            structs: [EMPTY_STRUCT; MAX_STRUCTS],
            nstructs: 0,
            main_script: Text::new(),
        }
    }
}

fn is_space(c: u8) -> bool { c == b' ' || c == b'\t' || c == b'\r' }
fn is_ident(c: u8) -> bool { c.is_ascii_alphanumeric() || c == b'-' || c == b'_' }

pub fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && is_space(s[a]) { a += 1; }
    while b > a && is_space(s[b - 1]) { b -= 1; }
    &s[a..b]
}

fn strip_cr(s: &[u8]) -> &[u8] {
    if s.last() == Some(&b'\r') { &s[..s.len() - 1] } else { s }
}

/// Strip `//` only outside quoted strings.
fn strip_comment(s: &[u8]) -> &[u8] {
    let mut quoted = false;
    let mut escaped = false;
    let mut i = 0;
    while i + 1 < s.len() {
        let c = s[i];
        if quoted {
            if escaped { escaped = false; }
            else if c == b'\\' { escaped = true; }
            else if c == b'"' { quoted = false; }
        } else if c == b'"' {
            quoted = true;
        } else if c == b'/' && s[i + 1] == b'/' {
            return trim(&s[..i]);
        }
        i += 1;
    }
    trim(s)
}

/// Remove YAML comments without modifying quoted scalars.
/// `#` starts a YAML comment only outside quotes and when it begins the line
/// or is preceded by whitespace. Single-quoted YAML escapes itself as `''`.
fn strip_yaml_comment(s: &[u8]) -> &[u8] {
    let mut quote = 0u8; // 0 = none, 1 = single, 2 = double
    let mut escaped = false;
    let mut i = 0usize;

    while i < s.len() {
        let c = s[i];
        if quote == 2 {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                quote = 0;
            }
        } else if quote == 1 {
            if c == b'\'' {
                if i + 1 < s.len() && s[i + 1] == b'\'' {
                    i += 1;
                } else {
                    quote = 0;
                }
            }
        } else {
            match c {
                b'"' => quote = 2,
                b'\'' => quote = 1,
                b'#' if i == 0 || s[i - 1].is_ascii_whitespace() => {
                    return trim(&s[..i]);
                }
                _ => {}
            }
        }
        i += 1;
    }
    trim(s)
}

fn ident(s: &[u8]) -> &[u8] {
    let mut n = 0;
    while n < s.len() && is_ident(s[n]) { n += 1; }
    &s[..n]
}

fn quoted_or_raw(v: &[u8]) -> &[u8] {
    let v = trim(v);
    if v.first() == Some(&b'"') {
        let mut i = 1;
        let mut escaped = false;
        while i < v.len() {
            let c = v[i];
            if escaped { escaped = false; }
            else if c == b'\\' { escaped = true; }
            else if c == b'"' { return &v[1..i]; }
            i += 1;
        }
        &v[1..]
    } else {
        let end = v.iter().position(|&c| c == b',' || c == b')').unwrap_or(v.len());
        trim(&v[..end])
    }
}

fn value_after<'a>(line: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let p = line.windows(key.len()).position(|w| w == key)?;
    Some(quoted_or_raw(&line[p + key.len()..]))
}

fn call_parts(src: &[u8]) -> Option<(&[u8], &[u8])> {
    let s = trim(src);
    let open = s.iter().position(|&c| c == b'(')?;
    let name = trim(&s[..open]);
    if name.is_empty() { return None; }

    // Find the matching ')' while respecting nested calls and quoted strings.
    let mut depth = 1usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut close = s.len();
    let mut i = open + 1;
    while i < s.len() {
        let ch = s[i];
        if quoted {
            if escaped { escaped = false; }
            else if ch == b'\\' { escaped = true; }
            else if ch == b'"' { quoted = false; }
        } else if ch == b'"' {
            quoted = true;
        } else if ch == b'(' {
            depth += 1;
        } else if ch == b')' {
            depth -= 1;
            if depth == 0 { close = i; break; }
        }
        i += 1;
    }
    if close == s.len() { return None; }

    let args = &s[open + 1..close];
    let mut depth = 0usize;
    quoted = false;
    escaped = false;
    let mut comma = args.len();
    for (i, &ch) in args.iter().enumerate() {
        if quoted {
            if escaped { escaped = false; }
            else if ch == b'\\' { escaped = true; }
            else if ch == b'"' { quoted = false; }
        } else if ch == b'"' {
            quoted = true;
        } else if ch == b'(' {
            depth += 1;
        } else if ch == b')' {
            depth = depth.saturating_sub(1);
        } else if ch == b',' && depth == 0 {
            comma = i;
            break;
        }
    }
    Some((name, trim(&args[..comma])))
}

fn raw_after<'a>(line: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let at = line.windows(key.len()).position(|window| window == key)?;
    Some(trim(&line[at + key.len()..]))
}

fn parse_script_call(src: &[u8]) -> ScriptCall {
    let mut out = ScriptCall::new();
    if let Some((name, arg)) = call_parts(src) {
        out.name.set(name);
        out.arg.set(arg);
    }
    out
}

fn parse_u64(src: &[u8]) -> u64 {
    let mut n = 0u64;
    for &c in trim(src) {
        if !c.is_ascii_digit() { break; }
        n = n.saturating_mul(10).saturating_add((c - b'0') as u64);
    }
    n
}

fn parse_hex_color(src: &[u8]) -> Option<[u8; 3]> {
    let s = trim(src);
    let hex = if s.first() == Some(&b'#') { &s[1..] } else { s };
    if hex.len() != 6 { return None; }
    let mut out = [0u8; 3];
    for i in 0..3 {
        let hi = hex_digit(hex[i * 2])?;
        let lo = hex_digit(hex[i * 2 + 1])?;
        out[i] = hi * 16 + lo;
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

fn is_component_kind(k: &[u8]) -> bool {
    k == b"alert" || k == b"text" || k == b"list" || k == b"button" || k == b"card"
}

fn add_comp(p: &mut Project, board: usize, parent: usize, kind: &[u8], name: &[u8]) -> Option<usize> {
    if p.ncomps >= MAX_COMPS { return None; }
    let idx = p.ncomps;
    let c = &mut p.comps[idx];
    *c = EMPTY_COMP;
    c.board = board;
    c.parent = parent;
    c.kind.set(kind);
    c.name.set(name);
    p.ncomps += 1;
    Some(idx)
}

fn add_item(c: &mut Comp, id: &[u8], label: &[u8]) -> Option<usize> {
    if c.nitems >= MAX_ITEMS { return None; }
    let i = c.nitems;
    c.items[i] = EMPTY_ITEM;
    c.items[i].id.set(id);
    c.items[i].text.set(label);
    c.nitems += 1;
    Some(i)
}

fn component_start(t: &[u8]) -> Option<(&[u8], &[u8])> {
    let open = t.iter().position(|&c| c == b'(')?;
    let kind = trim(&t[..open]);
    if !is_component_kind(kind) || kind == b"card" { return None; }
    let args = trim(&t[open + 1..]);
    let first = ident(args);
    let after = args.get(first.len()).copied();
    let name = if !first.is_empty() && (after.is_none() || after == Some(b')') || after == Some(b',') || after == Some(b'\n')) {
        first
    } else { &args[..0] };
    Some((kind, name))
}

fn parse_inline_component_props(c: &mut Comp, t: &[u8]) {
    if let Some(v) = value_after(t, b"config:title:") { c.label1.set(v); }
    if let Some(v) = value_after(t, b"config:text:") {
        if c.kind.get() == b"button" { c.label1.set(v); } else { c.label2.set(v); }
    }
    if let Some(v) = value_after(t, b"config:message:") { c.label2.set(v); }
    if let Some(v) = value_after(t, b".bgColor(") {
        if let Some(rgb) = parse_hex_color(v) { c.bg_color = rgb; c.has_bg_color = true; }
    }
    if let Some(v) = value_after(t, b".fgColor(") {
        if let Some(rgb) = parse_hex_color(v) { c.fg_color = rgb; c.has_fg_color = true; }
    }
    if let Some(v) = raw_after(t, b"event:") {
        if let Some((name, arg)) = call_parts(v) {
            if name == b"moveboard" { c.event.set(arg); }
        }
    }
}

pub fn parse_config(p: &mut Project, src: &[u8]) {
    for raw in src.split(|&c| c == b'\n') {
        let line = trim(strip_cr(raw));
        if let Some(v) = line.strip_prefix(b"name=") { p.title.set(trim(v)); }
    }
}

/// Parse board/component declarations and the documented one-line / bracketed model syntax.
pub fn parse_mb5(p: &mut Project, src: &[u8]) {
    let mut board: Option<usize> = None;
    let mut card_stack = [NO_PARENT; 8];
    let mut card_depth = 0usize;
    let mut last_component = NO_PARENT;
    let mut inline_model = false;
    let mut in_model = false;

    for raw in src.split(|&c| c == b'\n') {
        let t = strip_comment(strip_cr(raw));
        if t.is_empty() { continue; }
        if t.starts_with(b"@mainScript") || t.starts_with(b"@struct") { board = None; card_depth = 0; continue; }
        if t.starts_with(b"@mainBoard") { continue; }

        if t.starts_with(b"board ") && t.contains(&b'{') {
            let n = ident(trim(&t[6..]));
            if p.nboards < MAX_BOARDS {
                let idx = p.nboards;
                p.boards[idx].set(n);
                p.nboards += 1;
                board = Some(idx);
                if p.main_board.is_empty() { p.main_board.set(n); }
            }
            card_depth = 0;
            last_component = NO_PARENT;
            continue;
        }
        let b = match board { Some(v) => v, None => continue };

        if t.starts_with(b"card") && t.contains(&b'{') {
            let parent = if card_depth > 0 { card_stack[card_depth - 1] } else { NO_PARENT };
            if let Some(idx) = add_comp(p, b, parent, b"card", b"") {
                if card_depth < card_stack.len() { card_stack[card_depth] = idx; card_depth += 1; }
                last_component = idx;
            }
            continue;
        }
        if t == b"}" || t.starts_with(b"}") {
            if card_depth > 0 { card_depth -= 1; }
            else { board = None; }
            last_component = NO_PARENT;
            inline_model = false;
            in_model = false;
            continue;
        }

        if let Some((kind, name)) = component_start(t) {
            let parent = if card_depth > 0 { card_stack[card_depth - 1] } else { NO_PARENT };
            if let Some(idx) = add_comp(p, b, parent, kind, name) {
                last_component = idx;
                parse_inline_component_props(&mut p.comps[idx], t);
            }
            inline_model = false;
            in_model = false;
            continue;
        }

        if last_component != NO_PARENT && last_component < p.ncomps {
            let c = &mut p.comps[last_component];
            if t.starts_with(b".id(") {
                if let Some(v) = value_after(t, b".id(") { c.id.set(v); }
            } else if t.starts_with(b".display(") {
                c.hidden = t.windows(4).any(|w| w == b"hide");
            } else if t.starts_with(b".type(") {
                if let Some(v) = value_after(t, b".type(") {
                    c.primary = v == b"primary";
                    c.text_button = v == b"text";
                }
            } else if t.starts_with(b".width(") {
                if let Some(v) = value_after(t, b".width(") { c.width.set(v); }
            } else if t.starts_with(b".size(") {
                if let Some(v) = value_after(t, b".size(") { c.size.set(v); }
            } else if t.starts_with(b".radius(") {
                if let Some(v) = value_after(t, b".radius(") {
                    c.radius = parse_u64(v).min(64) as i32;
                    c.radius_custom = true;
                }
            } else if t.starts_with(b".bgColor(") {
                if let Some(v) = value_after(t, b".bgColor(") { if let Some(rgb) = parse_hex_color(v) { c.bg_color = rgb; c.has_bg_color = true; } }
            } else if t.starts_with(b".fgColor(") {
                if let Some(v) = value_after(t, b".fgColor(") { if let Some(rgb) = parse_hex_color(v) { c.fg_color = rgb; c.has_fg_color = true; } }
            } else if t.starts_with(b"config:title:") {
                if let Some(v) = value_after(t, b"config:title:") { c.label1.set(v); }
            } else if t.starts_with(b"config:text:") {
                if let Some(v) = value_after(t, b"config:text:") {
                    if c.kind.get() == b"button" { c.label1.set(v); } else { c.label2.set(v); }
                }
            } else if t.starts_with(b"config:message:") {
                if let Some(v) = value_after(t, b"config:message:") { c.label2.set(v); }
            } else if t.starts_with(b"event:") {
                if let Some(v) = raw_after(t, b"event:") {
                    let call = parse_script_call(v);
                    if call.name.get() == b"moveboard" { c.event.set(call.arg.get()); }
                }
            } else if t == b"[" || t.starts_with(b"[") {
                inline_model = true;
                in_model = false;
            } else if t == b"]" || t.starts_with(b"]") {
                inline_model = false;
                in_model = false;
            } else if inline_model && t == b"model:" {
                in_model = true;
            } else if inline_model && in_model {
                if let Some(colon) = t.iter().position(|&ch| ch == b':') {
                    let key = trim(&t[..colon]);
                    let value = quoted_or_raw(&t[colon + 1..]);
                    if !key.is_empty() && key.iter().all(|ch| ch.is_ascii_digit()) {
                        let _ = add_item(c, key, value);
                    }
                }
            }
        }
    }
    parse_structs(p, src);
    parse_scripts(p, src);
}

fn parse_structs(p: &mut Project, src: &[u8]) {
    let mut in_section = false;
    let mut current_struct = NO_PARENT;
    let mut last_comp = NO_PARENT;

    for raw in src.split(|&c| c == b'\n') {
        let t = strip_comment(strip_cr(raw));
        if t == b"@struct" { in_section = true; continue; }
        if !in_section || t.is_empty() { continue; }
        if t.starts_with(b"@mainScript") { break; }
        if t.starts_with(b"components ") && t.contains(&b'{') {
            if p.nstructs >= MAX_STRUCTS { current_struct = NO_PARENT; continue; }
            let name = ident(trim(&t[11..]));
            let idx = p.nstructs;
            p.structs[idx] = EMPTY_STRUCT;
            p.structs[idx].name.set(name);
            p.nstructs += 1;
            current_struct = idx;
            last_comp = NO_PARENT;
            continue;
        }
        if t == b"}" || t.starts_with(b"}") {
            current_struct = NO_PARENT;
            last_comp = NO_PARENT;
            continue;
        }
        if current_struct == NO_PARENT { continue; }
        if let Some((kind, name)) = component_start(t) {
            let sd = &mut p.structs[current_struct];
            if sd.ncomps < MAX_STRUCT_COMPS {
                let comp_idx = sd.ncomps;
                sd.comps[comp_idx] = EMPTY_COMP;
                sd.comps[comp_idx].kind.set(kind);
                sd.comps[comp_idx].name.set(name);
                parse_inline_component_props(&mut sd.comps[comp_idx], t);
                sd.ncomps += 1;
                last_comp = comp_idx;
            }
            continue;
        }
        if last_comp != NO_PARENT {
            let comp = &mut p.structs[current_struct].comps[last_comp];
            if t.starts_with(b".id(") { if let Some(v) = value_after(t, b".id(") { comp.id.set(v); } }
            else if t.starts_with(b".display(") { comp.hidden = t.windows(4).any(|w| w == b"hide"); }
            else if t.starts_with(b".type(") {
                if let Some(v) = value_after(t, b".type(") {
                    comp.primary = v == b"primary";
                    comp.text_button = v == b"text";
                }
            }
            else if t.starts_with(b".radius(") {
                if let Some(v) = value_after(t, b".radius(") {
                    comp.radius = parse_u64(v).min(64) as i32;
                    comp.radius_custom = true;
                }
            }
            else if t.starts_with(b".bgColor(") { if let Some(v) = value_after(t, b".bgColor(") { if let Some(rgb) = parse_hex_color(v) { comp.bg_color = rgb; comp.has_bg_color = true; } } }
            else if t.starts_with(b".fgColor(") { if let Some(v) = value_after(t, b".fgColor(") { if let Some(rgb) = parse_hex_color(v) { comp.fg_color = rgb; comp.has_fg_color = true; } } }
            else if t.starts_with(b"config:title:") { if let Some(v) = value_after(t, b"config:title:") { comp.label1.set(v); } }
            else if t.starts_with(b"config:text:") { if let Some(v) = value_after(t, b"config:text:") { if comp.kind.get() == b"button" { comp.label1.set(v); } else { comp.label2.set(v); } } }
            else if t.starts_with(b"config:message:") { if let Some(v) = value_after(t, b"config:message:") { comp.label2.set(v); } }
        }
    }
}


fn add_script(p: &mut Project, name: &[u8], branch: &[u8]) -> Option<usize> {
    if p.nscripts >= MAX_SCRIPTS { return None; }
    let i = p.nscripts;
    p.scripts[i] = EMPTY_SCRIPT;
    p.scripts[i].name.set(name);
    p.scripts[i].branch.set(branch);
    p.nscripts += 1;
    Some(i)
}

fn push_command(p: &mut Project, script: usize, cmd: ScriptCommand) {
    if script >= p.nscripts { return; }
    let s = &mut p.scripts[script];
    if s.ncommands < MAX_COMMANDS {
        s.commands[s.ncommands] = cmd;
        s.ncommands += 1;
    }
}

fn parse_script_command(t: &[u8]) -> Option<ScriptCommand> {
    let mut cmd = EMPTY_COMMAND;
    if t.starts_with(b"wait(") {
        let (_, arg) = call_parts(t)?;
        cmd.kind = CMD_WAIT;
        cmd.millis = parse_u64(arg);
        return Some(cmd);
    }
    if t.starts_with(b"setComponentConfig(") {
        let body = t.strip_prefix(b"setComponentConfig(")?;
        let close = body.iter().rposition(|&c| c == b')').unwrap_or(body.len());
        let args = &body[..close];
        let comma = args.iter().position(|&c| c == b',')?;
        cmd.kind = CMD_SET_DISPLAY;
        cmd.subject.set(trim(&args[..comma]));
        cmd.flag = !args.windows(b"display:hide".len()).any(|w| w == b"display:hide");
        return Some(cmd);
    }
    if t.starts_with(b"addComponent(") || t.starts_with(b"deleteComponent(") || t.starts_with(b"deleteCmponent(") {
        let add = t.starts_with(b"addComponent(");
        let call = call_parts(t)?;
        cmd.kind = if add { CMD_ADD_COMPONENT } else { CMD_DELETE_COMPONENT };
        cmd.subject.set(call.1);
        return Some(cmd);
    }
    let (name, arg) = call_parts(t)?;
    if name == b"moveboard" { return None; }
    cmd.kind = CMD_CALL;
    cmd.subject.set(name);
    cmd.arg.set(arg);
    Some(cmd)
}

/// Parse the independent @mainScript section. The first script is the startup script.
fn parse_scripts(p: &mut Project, src: &[u8]) {
    let mut in_section = false;
    let mut base_script = NO_PARENT;
    let mut active_script = NO_PARENT;
    let mut in_branch = false;

    for raw in src.split(|&c| c == b'\n') {
        let t = strip_comment(strip_cr(raw));
        if t.starts_with(b"@mainScript") { in_section = true; continue; }
        if !in_section || t.is_empty() { continue; }

        if t.starts_with(b"script ") && t.contains(&b'{') {
            let name = ident(trim(&t[7..]));
            if let Some(idx) = add_script(p, name, b"") {
                base_script = idx;
                active_script = idx;
                if p.main_script.is_empty() { p.main_script.set(name); }
            }
            in_branch = false;
            continue;
        }
        if active_script != NO_PARENT && t.starts_with(b"message(") {
            if let Some((_, arg)) = call_parts(t) {
                let base_name = p.scripts[base_script].name;
                active_script = add_script(p, base_name.get(), arg).unwrap_or(base_script);
                in_branch = true;
            }
            continue;
        }
        if t == b"}" || t.starts_with(b"}") {
            if in_branch {
                active_script = base_script;
                in_branch = false;
            } else {
                active_script = NO_PARENT;
                base_script = NO_PARENT;
            }
            continue;
        }
        if active_script == NO_PARENT { continue; }
        if t.starts_with(b".target(") {
            if let Some(v) = value_after(t, b".target(") {
                let s = &mut p.scripts[active_script];
                if s.ncommands > 0 { s.commands[s.ncommands - 1].target.set(v); }
            }
            continue;
        }
        if let Some(cmd) = parse_script_command(t) {
            push_command(p, active_script, cmd);
        }
    }
}

/// Resolve one component's YAML model.
fn resolve_component_yaml(comp: &mut Comp, src: &[u8]) {
    if comp.name.is_empty() { return; }
    let key = comp.name;
    let mut found = false;
    let mut section = 0u8; // 1=config, 2=model, 3=events
    let mut current_item: Option<usize> = None;
    let mut item_is_object = false;
    let mut close_is_object = false;

    for raw in src.split(|&c| c == b'\n') {
        let line = strip_cr(raw);
        if line.is_empty() { continue; }
        let indent = line.iter().take_while(|&&c| c == b' ').count();
        let t = strip_yaml_comment(trim(line));
        if t.is_empty() {
            continue;
        }
        if indent == 0 {
            if found { break; }
            if t.len() == key.get().len() + 1 && t.starts_with(key.get()) && t.last() == Some(&b':') {
                found = true;
            }
            continue;
        }
        if !found { continue; }

        if indent == 2 {
            section = if t == b"config:" { 1 } else if t == b"model:" { 2 } else if t == b"events:" { 3 } else { 0 };
            current_item = None;
            item_is_object = false;
            close_is_object = false;
            continue;
        }

        if section == 1 && indent >= 4 {
            if t.starts_with(b"target:") { continue; }
            if let Some(v) = t.strip_prefix(b"title:") { comp.label1.set(quoted_or_raw(v)); }
            else if let Some(v) = t.strip_prefix(b"message:") { comp.label2.set(quoted_or_raw(v)); }
            else if let Some(v) = t.strip_prefix(b"text:") {
                if comp.kind.get() == b"button" { comp.label1.set(quoted_or_raw(v)); }
                else { comp.label2.set(quoted_or_raw(v)); }
            }
        } else if section == 3 && indent >= 4 {
            if let Some(colon) = t.iter().position(|&c| c == b':') {
                let event_name = trim(&t[..colon]);
                let call = parse_script_call(&t[colon + 1..]);
                if event_name == b"close" { comp.close_event = call; }
            }
        } else if section == 2 {
            if indent == 4 {
                let colon = match t.iter().position(|&c| c == b':') { Some(v) => v, None => continue };
                let item_key = trim(&t[..colon]);
                let v = trim(&t[colon + 1..]);
                if item_key == b"close" {
                    if !v.is_empty() {
                        comp.close_label.set(quoted_or_raw(v));
                        close_is_object = false;
                    } else {
                        close_is_object = true;
                    }
                    current_item = None;
                    item_is_object = false;
                    continue;
                }
                close_is_object = false;
                if comp.kind.get() == b"list" {
                    if !v.is_empty() {
                        current_item = add_item(comp, item_key, quoted_or_raw(v));
                        item_is_object = false;
                    } else {
                        current_item = add_item(comp, item_key, b"");
                        item_is_object = true;
                    }
                } else {
                    current_item = None;
                    item_is_object = false;
                }
            } else if indent >= 6 {
                if close_is_object {
                    if let Some(v) = t.strip_prefix(b"text:") { comp.close_label.set(quoted_or_raw(v)); }
                    else if let Some(v) = t.strip_prefix(b"event:") { comp.close_event = parse_script_call(v); }
                } else if item_is_object && comp.kind.get() == b"list" {
                    if let Some(item_idx) = current_item {
                        if let Some(v) = t.strip_prefix(b"text:") { comp.items[item_idx].text.set(quoted_or_raw(v)); }
                        else if let Some(v) = t.strip_prefix(b"event:") { comp.items[item_idx].call = parse_script_call(v); }
                    }
                }
            }
        }
    }

    // Events can be declared before model IDs, so resolve the model-key mapping after parsing.
    found = false;
    let mut in_events = false;
    for raw in src.split(|&c| c == b'\n') {
        let line = strip_cr(raw);
        if line.is_empty() { continue; }
        let indent = line.iter().take_while(|&&c| c == b' ').count();
        let t = strip_yaml_comment(trim(line));
        if t.is_empty() {
            continue;
        }
        if indent == 0 {
            if found { break; }
            if t.len() == key.get().len() + 1 && t.starts_with(key.get()) && t.last() == Some(&b':') { found = true; }
            continue;
        }
        if !found { continue; }
        if indent == 2 {
            in_events = t == b"events:";
            continue;
        }
        if in_events && indent >= 4 {
            if let Some(colon) = t.iter().position(|&c| c == b':') {
                let event_id = trim(&t[..colon]);
                let call = parse_script_call(&t[colon + 1..]);
                if event_id == b"close" {
                    comp.close_event = call;
                } else {
                    for item_i in 0..comp.nitems {
                        if comp.items[item_i].id.get() == event_id {
                            comp.items[item_i].call = call;
                            break;
                        }
                    }
                }
            }
        }
    }
}

pub fn resolve_labels(p: &mut Project, src: &[u8]) {
    for i in 0..p.ncomps { resolve_component_yaml(&mut p.comps[i], src); }
    for si in 0..p.nstructs {
        for ci in 0..p.structs[si].ncomps {
            resolve_component_yaml(&mut p.structs[si].comps[ci], src);
        }
    }
}
