use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
enum Node {
    Raw(String),
    Block { header: String, body: Vec<Node> },
}

#[derive(Clone, Copy, Debug)]
enum CompareOp {
    Eq,
    Ne,
    Lt,
    Gt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signal {
    None,
    Break,
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(source: &str) -> Self {
        let cleaned = strip_comments(source);
        Self {
            chars: cleaned.chars().collect(),
            pos: 0,
        }
    }

    fn parse_program(&mut self, stop_on_brace: bool) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();

        loop {
            self.skip_whitespace();

            if self.eof() {
                if stop_on_brace {
                    return Err("'}' が不足しています".into());
                }
                break;
            }

            if self.peek() == Some('}') {
                if stop_on_brace {
                    self.pos += 1;
                    break;
                }
                return Err("対応する '{' がない '}' があります".into());
            }

            let start = self.pos;
            let mut paren = 0i32;
            let mut bracket = 0i32;
            let mut found_block = false;

            while !self.eof() {
                let c = self.chars[self.pos];
                match c {
                    '(' => paren += 1,
                    ')' => paren -= 1,
                    '[' => bracket += 1,
                    ']' => bracket -= 1,
                    '{' if paren == 0 && bracket == 0 => {
                        let header: String = self.chars[start..self.pos].iter().collect();
                        let header = header.trim().to_string();
                        if header.is_empty() {
                            return Err("'{' の前に命令がありません".into());
                        }
                        self.pos += 1;
                        let body = self.parse_program(true)?;
                        nodes.push(Node::Block { header, body });
                        found_block = true;
                        break;
                    }
                    '\n' if paren == 0 && bracket == 0 => {
                        let raw: String = self.chars[start..self.pos].iter().collect();
                        self.pos += 1;
                        if !raw.trim().is_empty() {
                            nodes.push(Node::Raw(raw.trim().to_string()));
                        }
                        found_block = true; // この文は処理済み
                        break;
                    }
                    '}' if paren == 0 && bracket == 0 => {
                        if stop_on_brace {
                            let raw: String = self.chars[start..self.pos].iter().collect();
                            if !raw.trim().is_empty() {
                                nodes.push(Node::Raw(raw.trim().to_string()));
                            }
                            self.pos += 1;
                            return Ok(nodes);
                        }
                        return Err("対応する '{' がない '}' があります".into());
                    }
                    _ => {}
                }
                self.pos += 1;
            }

            if found_block {
                continue;
            }

            if self.eof() {
                let raw: String = self.chars[start..self.pos].iter().collect();
                if !raw.trim().is_empty() {
                    nodes.push(Node::Raw(raw.trim().to_string()));
                }
            }
        }

        Ok(nodes)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

struct Interpreter {
    vars: HashMap<String, String>,
    consts: HashMap<String, String>,
    functions: HashMap<String, Vec<Node>>,
    hoisted_functions: HashSet<String>,
}

impl Interpreter {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
            consts: HashMap::new(),
            functions: HashMap::new(),
            hoisted_functions: HashSet::new(),
        }
    }

    fn run(&mut self, nodes: &[Node]) -> Result<(), String> {
        self.hoist_top_level_functions(nodes)?;
        self.exec_nodes(nodes, 0)?;
        Ok(())
    }

    // 通常の fun (name) { ... } は先に登録するので、定義より前から呼び出せる。
    // var[] / const[] / calc[] を含む動的な関数名は、その定義位置で展開して登録する。
    fn hoist_top_level_functions(&mut self, nodes: &[Node]) -> Result<(), String> {
        for node in nodes {
            let Node::Block { header, body } = node else {
                continue;
            };
            if header.contains("var[") || header.contains("const[") || header.contains("calc[") {
                continue;
            }
            if let Some(name) = parse_fun_definition_header(header)? {
                if self.functions.contains_key(&name) {
                    return Err(format!("関数 '{}' は重複定義できません", name));
                }
                self.functions.insert(name.clone(), body.clone());
                self.hoisted_functions.insert(name);
            }
        }
        Ok(())
    }

    fn exec_nodes(&mut self, nodes: &[Node], loop_depth: usize) -> Result<Signal, String> {
        for node in nodes {
            let signal = match node {
                Node::Raw(raw) => self.exec_raw(raw, loop_depth)?,
                Node::Block { header, body } => self.exec_block(header, body, loop_depth)?,
            };

            if signal == Signal::Break {
                return Ok(Signal::Break);
            }
        }
        Ok(Signal::None)
    }

    fn exec_block(
        &mut self,
        header: &str,
        body: &[Node],
        loop_depth: usize,
    ) -> Result<Signal, String> {
        let expanded = self.expand_all(header)?;
        let header = expanded.trim();

        if let Some(name) = parse_fun_definition_header(header)? {
            if self.hoisted_functions.remove(&name) {
                return Ok(Signal::None);
            }
            if self.functions.contains_key(&name) {
                return Err(format!("関数 '{}' は重複定義できません", name));
            }
            self.functions.insert(name, body.to_vec());
            return Ok(Signal::None);
        }

        if let Some(condition) = header.strip_prefix("if") {
            if !header_after_keyword_is_valid(header, "if") {
                return Err(format!("不明なブロック命令です: {}", header));
            }
            let (left, op, right) = split_condition(condition.trim())
                .ok_or_else(|| format!("if の条件を解析できません: {}", condition.trim()))?;
            if compare(&left, op, &right) {
                return self.exec_nodes(body, loop_depth);
            }
            return Ok(Signal::None);
        }

        Err(format!("不明なブロック命令です: {}", header))
    }

    fn exec_raw(&mut self, raw: &str, loop_depth: usize) -> Result<Signal, String> {
        let expanded = self.expand_all(raw)?;
        let stmt = expanded.trim();
        if stmt.is_empty() {
            return Ok(Signal::None);
        }

        if stmt == "break" {
            return Ok(if loop_depth > 0 {
                Signal::Break
            } else {
                Signal::None
            });
        }

        if let Some(rest) = strip_command(stmt, "var.set") {
            let (name, value) = parse_name_value(rest, "var.set")?;
            if !self.vars.contains_key(&name) {
                self.vars.insert(name, value);
            }
            return Ok(Signal::None);
        }

        if let Some(rest) = strip_command(stmt, "var.edit") {
            let (name, value) = parse_name_value(rest, "var.edit")?;
            if self.vars.contains_key(&name) {
                self.vars.insert(name, value);
            }
            return Ok(Signal::None);
        }

        if let Some(rest) = strip_command(stmt, "const.set") {
            let (name, value) = parse_name_value(rest, "const.set")?;
            if !self.consts.contains_key(&name) {
                self.consts.insert(name, value);
            }
            return Ok(Signal::None);
        }

        if let Some(content) = parse_call(stmt, "print")? {
            println!("{}", content);
            return Ok(Signal::None);
        }

        if let Some(content) = parse_call(stmt, "wait")? {
            let duration = parse_duration(content.trim())?;
            thread::sleep(duration);
            return Ok(Signal::None);
        }

        if let Some((left, op, right, inline_body)) = parse_inline_if(stmt)? {
            if compare(&left, op, &right) {
                let mut parser = Parser::new(&inline_body);
                let nodes = parser.parse_program(false)?;
                return self.exec_nodes(&nodes, loop_depth);
            }
            return Ok(Signal::None);
        }

        if let Some(name) = parse_fun_call(stmt)? {
            return self.call_function(&name, loop_depth);
        }

        if let Some(name) = parse_for_call(stmt)? {
            loop {
                match self.call_function(&name, loop_depth + 1)? {
                    Signal::Break => break,
                    Signal::None => {}
                }
            }
            return Ok(Signal::None);
        }

        Err(format!("不明な命令です: {}", stmt))
    }

    fn call_function(&mut self, name: &str, loop_depth: usize) -> Result<Signal, String> {
        let expanded_name = self.expand_all(name)?.trim().to_string();
        let body = self
            .functions
            .get(&expanded_name)
            .cloned()
            .ok_or_else(|| format!("関数 '{}' は定義されていません", expanded_name))?;
        self.exec_nodes(&body, loop_depth)
    }

    fn expand_all(&self, input: &str) -> Result<String, String> {
        let mut current = input.to_string();
        for _ in 0..256 {
            let (next, changed) = self.expand_pass(&current)?;
            if !changed || next == current {
                return Ok(next);
            }
            current = next;
        }
        Err("展開回数が上限を超えました。循環参照の可能性があります".into())
    }

    fn expand_pass(&self, input: &str) -> Result<(String, bool), String> {
        let chars: Vec<char> = input.chars().collect();
        let mut out = String::new();
        let mut i = 0usize;
        let mut changed = false;

        while i < chars.len() {
            let kind = if starts_at(&chars, i, "var[") {
                Some("var")
            } else if starts_at(&chars, i, "const[") {
                Some("const")
            } else if starts_at(&chars, i, "calc[") {
                Some("calc")
            } else {
                None
            };

            let Some(kind) = kind else {
                out.push(chars[i]);
                i += 1;
                continue;
            };

            let prefix_len = kind.chars().count();
            let open = i + prefix_len;
            let Some(close) = find_matching_square(&chars, open) else {
                return Err(format!("{}[ ... ] の ']' が不足しています", kind));
            };

            let inner: String = chars[open + 1..close].iter().collect();
            let inner = self.expand_all(&inner)?;

            match kind {
                "var" => {
                    let key = inner.trim();
                    if let Some(value) = self.vars.get(key) {
                        out.push_str(value);
                        changed = true;
                    } else {
                        out.push_str(&chars[i..=close].iter().collect::<String>());
                    }
                }
                "const" => {
                    let key = inner.trim();
                    if let Some(value) = self.consts.get(key) {
                        out.push_str(value);
                        changed = true;
                    } else {
                        out.push_str(&chars[i..=close].iter().collect::<String>());
                    }
                }
                "calc" => {
                    let value = eval_calc(&inner)?;
                    out.push_str(&format_number(value));
                    changed = true;
                }
                _ => unreachable!(),
            }

            i = close + 1;
        }

        Ok((out, changed))
    }
}

fn strip_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once('#').map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

fn header_after_keyword_is_valid(header: &str, keyword: &str) -> bool {
    if !header.starts_with(keyword) {
        return false;
    }
    let rest = &header[keyword.len()..];
    rest.is_empty()
        || rest.starts_with('(')
        || rest.chars().next().is_some_and(|c| c.is_whitespace())
}

fn parse_fun_definition_header(header: &str) -> Result<Option<String>, String> {
    if !header_after_keyword_is_valid(header, "fun") {
        return Ok(None);
    }
    let rest = header[3..].trim_start();
    if !rest.starts_with('(') {
        return Ok(None);
    }
    let (inside, end) = read_balanced_str(rest, 0, '(', ')')?;
    if !rest[end..].trim().is_empty() {
        return Ok(None);
    }
    let name = inside.trim();
    if name.is_empty() {
        return Err("fun の関数名が空です".into());
    }
    Ok(Some(name.to_string()))
}

fn split_condition(header: &str) -> Option<(String, CompareOp, String)> {
    let chars: Vec<char> = header.chars().collect();
    let mut paren = 0i32;
    let mut bracket = 0i32;
    let mut i = 0usize;

    while i < chars.len() {
        if paren == 0 && bracket == 0 {
            if i + 1 < chars.len() && chars[i] == '!' && chars[i + 1] == '=' {
                return Some((
                    chars[..i].iter().collect::<String>().trim().to_string(),
                    CompareOp::Ne,
                    chars[i + 2..].iter().collect::<String>().trim().to_string(),
                ));
            }
            if chars[i] == '=' {
                return Some((
                    chars[..i].iter().collect::<String>().trim().to_string(),
                    CompareOp::Eq,
                    chars[i + 1..].iter().collect::<String>().trim().to_string(),
                ));
            }
            if chars[i] == '<' {
                return Some((
                    chars[..i].iter().collect::<String>().trim().to_string(),
                    CompareOp::Lt,
                    chars[i + 1..].iter().collect::<String>().trim().to_string(),
                ));
            }
            if chars[i] == '>' {
                return Some((
                    chars[..i].iter().collect::<String>().trim().to_string(),
                    CompareOp::Gt,
                    chars[i + 1..].iter().collect::<String>().trim().to_string(),
                ));
            }
        }

        match chars[i] {
            '(' => paren += 1,
            ')' => paren -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            _ => {}
        }
        i += 1;
    }
    None
}

fn compare(left: &str, op: CompareOp, right: &str) -> bool {
    match op {
        CompareOp::Eq => left.trim() == right.trim(),
        CompareOp::Ne => left.trim() != right.trim(),
        CompareOp::Lt => match (left.trim().parse::<f64>(), right.trim().parse::<f64>()) {
            (Ok(a), Ok(b)) => a < b,
            _ => false,
        },
        CompareOp::Gt => match (left.trim().parse::<f64>(), right.trim().parse::<f64>()) {
            (Ok(a), Ok(b)) => a > b,
            _ => false,
        },
    }
}

fn parse_inline_if(stmt: &str) -> Result<Option<(String, CompareOp, String, String)>, String> {
    if !header_after_keyword_is_valid(stmt, "if") {
        return Ok(None);
    }
    let rest = stmt[2..].trim_start();
    let chars: Vec<char> = rest.chars().collect();
    let mut bracket = 0i32;
    let mut candidate = None;

    for (i, c) in chars.iter().enumerate() {
        match *c {
            '[' => bracket += 1,
            ']' => bracket -= 1,
            '(' if bracket == 0 => {
                candidate = Some(i);
                break;
            }
            _ => {}
        }
    }

    let Some(body_open) = candidate else {
        return Ok(None);
    };

    let condition_text: String = chars[..body_open].iter().collect();
    let Some((left, op, right)) = split_condition(condition_text.trim()) else {
        return Ok(None);
    };

    let rest_bytes_index = char_index_to_byte(rest, body_open);
    let (body, end) = read_balanced_str(rest, rest_bytes_index, '(', ')')?;
    if !rest[end..].trim().is_empty() {
        return Ok(None);
    }

    Ok(Some((left, op, right, body.trim().to_string())))
}

fn strip_command<'a>(stmt: &'a str, command: &str) -> Option<&'a str> {
    if !stmt.starts_with(command) {
        return None;
    }
    let rest = &stmt[command.len()..];
    if rest.is_empty() || rest.chars().next().is_some_and(|c| c.is_whitespace()) {
        Some(rest.trim_start())
    } else {
        None
    }
}

fn parse_name_value(rest: &str, command: &str) -> Result<(String, String), String> {
    let Some(paren_pos) = find_top_level_char(rest, '(') else {
        return Err(format!("{} は '{} 名前 (値)' の形式で指定してください", command, command));
    };
    let name = rest[..paren_pos].trim().to_string();
    if name.is_empty() {
        return Err(format!("{} の名前が空です", command));
    }
    let (value, end) = read_balanced_str(rest, paren_pos, '(', ')')?;
    if !rest[end..].trim().is_empty() {
        return Err(format!("{} の値の後ろに解釈できない文字があります", command));
    }
    Ok((name, value.to_string()))
}

fn parse_call<'a>(stmt: &'a str, name: &str) -> Result<Option<&'a str>, String> {
    if !stmt.starts_with(name) {
        return Ok(None);
    }
    let rest = &stmt[name.len()..];
    if !(rest.starts_with('(')
        || rest.chars().next().is_some_and(|c| c.is_whitespace()))
    {
        return Ok(None);
    }

    let trimmed = rest.trim_start();
    if !trimmed.starts_with('(') {
        return Ok(None);
    }
    let (inside, end) = read_balanced_str(trimmed, 0, '(', ')')?;
    if !trimmed[end..].trim().is_empty() {
        return Err(format!("{}(...) の後ろに解釈できない文字があります", name));
    }
    Ok(Some(inside))
}

fn parse_fun_call(stmt: &str) -> Result<Option<String>, String> {
    parse_named_call(stmt, "fun")
}

fn parse_for_call(stmt: &str) -> Result<Option<String>, String> {
    parse_named_call(stmt, "for")
}

fn parse_named_call(stmt: &str, command: &str) -> Result<Option<String>, String> {
    if !header_after_keyword_is_valid(stmt, command) {
        return Ok(None);
    }
    let rest = stmt[command.len()..].trim_start();

    if rest.starts_with('(') {
        let (inside, end) = read_balanced_str(rest, 0, '(', ')')?;
        if !rest[end..].trim().is_empty() {
            return Err(format!("{}(...) の後ろに解釈できない文字があります", command));
        }
        let name = inside.trim();
        if name.is_empty() {
            return Err(format!("{} の関数名が空です", command));
        }
        return Ok(Some(name.to_string()));
    }

    let name = rest.trim();
    if name.is_empty() {
        return Err(format!("{} の関数名が空です", command));
    }
    Ok(Some(name.to_string()))
}

fn find_top_level_char(s: &str, wanted: char) -> Option<usize> {
    let mut bracket = 0i32;
    for (idx, c) in s.char_indices() {
        if c == wanted && bracket == 0 {
            return Some(idx);
        }
        match c {
            '[' => bracket += 1,
            ']' => bracket -= 1,
            _ => {}
        }
    }
    None
}

fn read_balanced_str<'a>(
    s: &'a str,
    start: usize,
    open: char,
    close: char,
) -> Result<(&'a str, usize), String> {
    if s.get(start..).and_then(|x| x.chars().next()) != Some(open) {
        return Err(format!("'{}' が必要です", open));
    }

    let mut depth = 0i32;
    let mut inner_start = None;
    for (rel, c) in s[start..].char_indices() {
        let idx = start + rel;
        if c == open {
            depth += 1;
            if depth == 1 {
                inner_start = Some(idx + c.len_utf8());
            }
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                let begin = inner_start.expect("balanced parser state");
                return Ok((&s[begin..idx], idx + c.len_utf8()));
            }
        }
    }
    Err(format!("'{}' が閉じられていません", open))
}

fn char_index_to_byte(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn parse_duration(text: &str) -> Result<Duration, String> {
    let units = ["ms", "us", "ns", "s", "m", "h"];
    for unit in units {
        if let Some(num) = text.strip_suffix(unit) {
            let value: f64 = num
                .trim()
                .parse()
                .map_err(|_| format!("wait の時間 '{}' を数値として解釈できません", text))?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("wait の時間 '{}' は 0 以上で指定してください", text));
            }
            let seconds = match unit {
                "ns" => value / 1_000_000_000.0,
                "us" => value / 1_000_000.0,
                "ms" => value / 1_000.0,
                "s" => value,
                "m" => value * 60.0,
                "h" => value * 3600.0,
                _ => unreachable!(),
            };
            return Ok(Duration::from_secs_f64(seconds));
        }
    }
    Err(format!(
        "wait の時間 '{}' に単位がありません (ns/us/ms/s/m/h)",
        text
    ))
}

fn starts_at(chars: &[char], index: usize, needle: &str) -> bool {
    let needle: Vec<char> = needle.chars().collect();
    index + needle.len() <= chars.len() && chars[index..index + needle.len()] == needle[..]
}

fn find_matching_square(chars: &[char], open_index: usize) -> Option<usize> {
    if chars.get(open_index) != Some(&'[') {
        return None;
    }
    let mut depth = 0i32;
    for (i, c) in chars.iter().enumerate().skip(open_index) {
        if *c == '[' {
            depth += 1;
        } else if *c == ']' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

struct CalcParser {
    chars: Vec<char>,
    pos: usize,
}

impl CalcParser {
    fn new(s: &str) -> Self {
        Self {
            chars: s.chars().collect(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<f64, String> {
        let value = self.expr()?;
        self.skip_ws();
        if self.pos != self.chars.len() {
            return Err(format!(
                "calc の式を解析できません: {}",
                self.chars[self.pos..].iter().collect::<String>()
            ));
        }
        if !value.is_finite() {
            return Err("calc の結果が有限の数値ではありません".into());
        }
        Ok(value)
    }

    fn expr(&mut self) -> Result<f64, String> {
        let mut value = self.term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    value += self.term()?;
                }
                Some('-') => {
                    self.pos += 1;
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn term(&mut self) -> Result<f64, String> {
        let mut value = self.factor()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    value *= self.factor()?;
                }
                Some('/') => {
                    self.pos += 1;
                    let rhs = self.factor()?;
                    if rhs == 0.0 {
                        return Err("calc で 0 除算はできません".into());
                    }
                    value /= rhs;
                }
                Some('%') => {
                    self.pos += 1;
                    let rhs = self.factor()?;
                    if rhs == 0.0 {
                        return Err("calc で 0 による剰余は計算できません".into());
                    }
                    value %= rhs;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn factor(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek() {
            Some('+') => {
                self.pos += 1;
                self.factor()
            }
            Some('-') => {
                self.pos += 1;
                Ok(-self.factor()?)
            }
            Some('(') => {
                self.pos += 1;
                let value = self.expr()?;
                self.skip_ws();
                if self.peek() != Some(')') {
                    return Err("calc の ')' が不足しています".into());
                }
                self.pos += 1;
                Ok(value)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            _ => Err("calc には数値と + - * / % ( ) のみ使用できます".into()),
        }
    }

    fn number(&mut self) -> Result<f64, String> {
        self.skip_ws();
        let start = self.pos;
        let mut seen_dot = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else if c == '.' && !seen_dot {
                seen_dot = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map_err(|_| format!("calc の数値 '{}' を解釈できません", text))
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
}

fn eval_calc(expr: &str) -> Result<f64, String> {
    CalcParser::new(expr).parse()
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{:.0}", value)
    } else {
        value.to_string()
    }
}

fn main() {
    let mut args = env::args();
    let exe = args.next().unwrap_or_else(|| "warp4-script".into());
    let Some(path) = args.next() else {
        eprintln!("使い方: {} <script-file>", exe);
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("指定できるスクリプトファイルは1つだけです");
        std::process::exit(2);
    }

    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", path, e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(&source);
    let program = match parser.parse_program(false) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("構文エラー: {}", e);
            std::process::exit(1);
        }
    };

    let mut interpreter = Interpreter::new();
    if let Err(e) = interpreter.run(&program) {
        eprintln!("実行エラー: {}", e);
        std::process::exit(1);
    }
}