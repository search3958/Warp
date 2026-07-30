use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Node {
    tags: Vec<String>,
    classes: Vec<String>,
    props: Vec<(String, String)>,
    children: Vec<Node>,
}

#[derive(Debug)]
struct UiParser {
    chars: Vec<char>,
    pos: usize,
}

impl UiParser {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<Vec<Node>, String> {
        self.parse_nodes(false)
    }

    fn parse_nodes(&mut self, nested: bool) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        loop {
            self.skip_space_and_comments();
            if self.eof() {
                if nested {
                    return Err("閉じる `}` がありません".into());
                }
                break;
            }
            if self.peek() == Some('}') {
                if !nested {
                    return Err(format!("{}文字目に余分な `}}` があります", self.pos + 1));
                }
                self.pos += 1;
                break;
            }

            let (tags, classes) = self.parse_selector()?;
            self.skip_space_and_comments();
            match self.peek() {
                Some('(') => {
                    if tags.len() != 1 || !classes.is_empty() {
                        return Err("プロパティ呼び出しの名前が不正です".into());
                    }
                    let value = self.parse_call_value()?;
                    return Err(format!(
                        "ルートではプロパティ `{}` を使えません（値: {value}）",
                        tags[0]
                    ));
                }
                Some('{') => {
                    self.pos += 1;
                    let mut node = Node {
                        tags,
                        classes,
                        props: Vec::new(),
                        children: Vec::new(),
                    };
                    loop {
                        self.skip_space_and_comments();
                        if self.peek() == Some('}') {
                            self.pos += 1;
                            break;
                        }
                        if self.eof() {
                            return Err("閉じる `}` がありません".into());
                        }
                        let saved = self.pos;
                        let (names, inner_classes) = self.parse_selector()?;
                        self.skip_space_and_comments();
                        if self.peek() == Some('(') {
                            if names.len() != 1 || !inner_classes.is_empty() {
                                return Err("プロパティ呼び出しの名前が不正です".into());
                            }
                            let value = self.parse_call_value()?;
                            node.props.push((names[0].clone(), value));
                        } else {
                            self.pos = saved;
                            let mut parsed = self.parse_nodes(true)?;
                            node.children.append(&mut parsed);
                            break;
                        }
                    }
                    nodes.push(node);
                }
                other => {
                    return Err(format!(
                        "{}文字目: `{{` または `(` が必要です（実際: {other:?}）",
                        self.pos + 1
                    ));
                }
            }
        }
        Ok(nodes)
    }

    fn parse_selector(&mut self) -> Result<(Vec<String>, Vec<String>), String> {
        let mut tags = Vec::new();
        let mut classes = Vec::new();
        tags.push(self.parse_ident()?);
        loop {
            if self.peek() == Some('.') {
                self.pos += 1;
                classes.push(self.parse_ident()?);
            } else if self.peek() == Some(',') {
                self.pos += 1;
                self.skip_space_and_comments();
                tags.push(self.parse_ident()?);
            } else {
                break;
            }
        }
        Ok((tags, classes))
    }

    fn parse_ident(&mut self) -> Result<String, String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '-' || c == '_') {
            self.pos += 1;
        }
        if self.pos == start {
            Err(format!("{}文字目: 名前が必要です", self.pos + 1))
        } else {
            Ok(self.chars[start..self.pos].iter().collect())
        }
    }

    fn parse_call_value(&mut self) -> Result<String, String> {
        self.expect('(')?;
        self.skip_space_and_comments();
        self.expect('"')?;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            match c {
                '"' => {
                    self.skip_space_and_comments();
                    self.expect(')')?;
                    return Ok(out);
                }
                '\\' => {
                    let escaped = self
                        .peek()
                        .ok_or_else(|| "文字列のエスケープが途中で終わっています".to_string())?;
                    self.pos += 1;
                    out.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        other => other,
                    });
                }
                other => out.push(other),
            }
        }
        Err("文字列が閉じられていません".into())
    }

    fn skip_space_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.pos += 1;
            }
            if self.peek() == Some('/') && self.chars.get(self.pos + 1) == Some(&'/') {
                while !matches!(self.peek(), None | Some('\n')) {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("{}文字目: `{expected}` が必要です", self.pos + 1))
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

#[derive(Debug, Clone)]
enum SectionKind {
    Click,
    Function,
}

#[derive(Debug, Clone)]
struct ScriptSection {
    kind: SectionKind,
    name: String,
    actions: Vec<(String, String)>,
}

fn parse_script(source: &str, file: &Path) -> Result<Vec<ScriptSection>, String> {
    let mut sections: Vec<ScriptSection> = Vec::new();
    for (index, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let inside = &line[1..line.len() - 1];
            let (kind, name) = inside.split_once('=').ok_or_else(|| {
                format!(
                    "{}:{}: セクションには `=` が必要です",
                    file.display(),
                    index + 1
                )
            })?;
            let kind = match kind.trim() {
                "onClick" => SectionKind::Click,
                "fun" => SectionKind::Function,
                other => {
                    return Err(format!(
                        "{}:{}: 不明なセクション `{other}`",
                        file.display(),
                        index + 1
                    ));
                }
            };
            sections.push(ScriptSection {
                kind,
                name: name.trim().to_string(),
                actions: Vec::new(),
            });
        } else {
            let (left, right) = line.split_once('=').ok_or_else(|| {
                format!("{}:{}: 命令には `=` が必要です", file.display(), index + 1)
            })?;
            let current = sections.last_mut().ok_or_else(|| {
                format!(
                    "{}:{}: 命令より前にセクションが必要です",
                    file.display(),
                    index + 1
                )
            })?;
            current
                .actions
                .push((left.trim().to_string(), right.trim().to_string()));
        }
    }
    Ok(sections)
}

fn ini(source: &str) -> BTreeMap<String, String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                None
            } else {
                line.split_once('=')
                    .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
            }
        })
        .collect()
}

fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.props
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn js_string(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn node_classes(node: &Node, primary: &str) -> String {
    let mut classes = vec!["w3-element".to_string(), format!("w3-{primary}")];
    classes.extend(node.tags.iter().skip(1).map(|tag| format!("w3-{tag}")));
    classes.extend(node.classes.iter().cloned());
    classes.join(" ")
}

fn render_children(nodes: &[Node], out: &mut String) {
    for node in nodes {
        render_node(node, out);
    }
}

fn render_node(node: &Node, out: &mut String) {
    let primary = node.tags.first().map(String::as_str).unwrap_or("div");
    if primary == "config" {
        return;
    }
    let classes = node_classes(node, primary);
    let text = prop(node, "text").unwrap_or("");
    let text_html = escape_html(text).replace('\n', "<br>");
    let raw_text = escape_html(text);
    let marker = if node.tags.iter().any(|tag| tag == "scroll-point") {
        " data-w3-scroll-point=\"true\""
    } else {
        ""
    };

    match primary {
        "text" => {
            out.push_str(&format!("<p class=\"{classes}\"{marker}>{text_html}"));
            render_children(&node.children, out);
            out.push_str("</p>");
        }
        "detail" => {
            out.push_str(&format!("<div class=\"{classes}\"{marker}>{text_html}"));
            render_children(&node.children, out);
            out.push_str("</div>");
        }
        "head" => {
            out.push_str(&format!("<h2 class=\"{classes}\"{marker}>{text_html}"));
            render_children(&node.children, out);
            out.push_str("</h2>");
        }
        "button" => {
            let button_type = prop(node, "type").unwrap_or("normal");
            out.push_str(&format!(
                "<button type=\"button\" class=\"{classes} w3-button-{button_type}\"{marker}>{text_html}"
            ));
            render_children(&node.children, out);
            out.push_str("</button>");
        }
        "input" => {
            out.push_str(&format!(
                "<input class=\"{classes}\" value=\"{}\"{marker}>",
                escape_html(text)
            ));
        }
        "textarea" => {
            out.push_str(&format!(
                "<textarea class=\"{classes}\"{marker}>{raw_text}</textarea>"
            ));
        }
        "switch" => {
            let checked = if prop(node, "default") == Some("true") {
                " checked"
            } else {
                ""
            };
            out.push_str(&format!(
                "<label class=\"{classes}\"{marker}><input type=\"checkbox\"{checked}><span></span></label>"
            ));
        }
        "code" => {
            out.push_str(&format!(
                "<pre class=\"{classes}\"{marker}><code>{raw_text}</code></pre>"
            ));
        }
        "space" => out.push_str(&format!("<div class=\"{classes}\"{marker}></div>")),
        "scroll-point" => {
            out.push_str(&format!(
                "<div class=\"{classes}\" data-w3-scroll-point=\"true\"></div>"
            ));
        }
        "list-box" => {
            if !text.is_empty() {
                out.push_str(&format!("<div class=\"w3-list-title\">{text_html}</div>"));
            }
            out.push_str(&format!("<ul class=\"{classes}\"{marker}>"));
            render_children(&node.children, out);
            out.push_str("</ul>");
        }
        "list" => {
            out.push_str(&format!(
                "<li class=\"{classes}\"{marker}><span>{text_html}</span>"
            ));
            render_children(&node.children, out);
            out.push_str("</li>");
        }
        "tab" => {
            let control = prop(node, "control").unwrap_or("top");
            out.push_str(&format!(
                "<section class=\"{classes} w3-tab-{control}\" data-w3-tabs{marker}><div class=\"w3-tab-controls\" role=\"tablist\">"
            ));
            for (index, child) in node.children.iter().enumerate() {
                if child.tags.first().map(String::as_str) == Some("content") {
                    let label = prop(child, "text").unwrap_or("Tab");
                    out.push_str(&format!(
                        "<button type=\"button\" role=\"tab\" data-w3-tab-button=\"{index}\">{}</button>",
                        escape_html(label)
                    ));
                }
            }
            out.push_str("</div><div class=\"w3-tab-pages\">");
            for (index, child) in node.children.iter().enumerate() {
                if child.tags.first().map(String::as_str) == Some("content") {
                    out.push_str(&format!(
                        "<div class=\"w3-tab-page\" role=\"tabpanel\" data-w3-tab-page=\"{index}\">"
                    ));
                    render_children(&child.children, out);
                    out.push_str("</div>");
                }
            }
            out.push_str("</div></section>");
        }
        "content" => {
            out.push_str(&format!("<div class=\"{classes}\"{marker}>"));
            render_children(&node.children, out);
            out.push_str("</div>");
        }
        tag => {
            let semantic = match tag {
                "toolbar" => "nav",
                "card" => "section",
                "flex" => "div",
                _ => "div",
            };
            out.push_str(&format!("<{semantic} class=\"{classes}\"{marker}>"));
            if !text.is_empty() {
                out.push_str(&text_html);
            }
            render_children(&node.children, out);
            out.push_str(&format!("</{semantic}>"));
        }
    }
}

fn collect_config(nodes: &[Node], title: &mut Option<String>, scripts: &mut BTreeSet<String>) {
    for node in nodes {
        if node.tags.first().map(String::as_str) == Some("config") {
            if let Some(value) = prop(node, "title") {
                *title = Some(value.to_string());
            }
            for (key, value) in &node.props {
                if key == "script" {
                    scripts.insert(value.clone());
                }
            }
        }
    }
}

fn variable_names(sections: &[ScriptSection]) -> BTreeSet<String> {
    let commands = ["scroll", "print", "screen", "wait", "fun"];
    let mut names = BTreeSet::new();
    for section in sections {
        for (left, _) in &section.actions {
            let words: Vec<_> = left.split_whitespace().collect();
            if words.len() == 1 && !commands.contains(&words[0]) {
                names.insert(words[0].to_string());
            } else if words.first() == Some(&"getText") && words.len() == 2 {
                names.insert(words[1].to_string());
            }
        }
    }
    names
}

fn value_expr(value: &str, variables: &BTreeSet<String>) -> String {
    let trimmed = value.trim();
    if variables.contains(trimmed) {
        format!("state[{}]", js_string(trimmed))
    } else if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        js_string(&trimmed[1..trimmed.len() - 1])
    } else if trimmed.parse::<f64>().is_ok() {
        trimmed.to_string()
    } else {
        js_string(trimmed)
    }
}

fn render_actions(
    actions: &[(String, String)],
    variables: &BTreeSet<String>,
) -> Result<String, String> {
    let mut out = String::new();
    for (left, value) in actions {
        let words: Vec<_> = left.split_whitespace().collect();
        match words.as_slice() {
            ["scroll"] => {
                if matches!(value.as_str(), "+1" | "-1") {
                    out.push_str(&format!("  scrollRelative({value});\n"));
                } else {
                    out.push_str(&format!("  scrollToPoint({});\n", js_string(value)));
                }
            }
            ["print"] => out.push_str(&format!(
                "  console.log({});\n",
                js_string(value.trim_matches('"'))
            )),
            ["screen"] => out.push_str(&format!("  showScreen({});\n", js_string(value))),
            ["wait"] => {
                let milliseconds = value
                    .strip_suffix("ms")
                    .unwrap_or(value)
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| {
                        format!("wait の値 `{value}` は `10ms` の形式で指定してください")
                    })?;
                out.push_str(&format!("  await delay({milliseconds});\n"));
            }
            ["fun"] => out.push_str(&format!("  await functions[{}]();\n", js_string(value))),
            ["setText", target] => out.push_str(&format!(
                "  setElementText({}, {});\n",
                js_string(target),
                value_expr(value, variables)
            )),
            ["getText", variable] => out.push_str(&format!(
                "  state[{}] = getElementText({});\n",
                js_string(variable),
                js_string(value)
            )),
            [variable] => {
                if let Some(delta) = value.strip_prefix('+') {
                    if delta.parse::<f64>().is_ok() {
                        out.push_str(&format!(
                            "  state[{}] = (Number(state[{}]) || 0) + {delta};\n",
                            js_string(variable),
                            js_string(variable)
                        ));
                    } else {
                        return Err(format!("`{left} = {value}` の加算値が不正です"));
                    }
                } else if value.starts_with('-') && value[1..].parse::<f64>().is_ok() {
                    out.push_str(&format!(
                        "  state[{}] = (Number(state[{}]) || 0) {value};\n",
                        js_string(variable),
                        js_string(variable)
                    ));
                } else {
                    out.push_str(&format!(
                        "  state[{}] = {};\n",
                        js_string(variable),
                        value_expr(value, variables)
                    ));
                }
            }
            _ => return Err(format!("不明な命令 `{left}`")),
        }
    }
    Ok(out)
}

fn generate_js(
    sections: &[ScriptSection],
    variables: &BTreeSet<String>,
    start_screen: &str,
) -> Result<String, String> {
    let mut out = String::from(
        r#""use strict";

const state = Object.create(null);
const delay = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
const namedElement = name => document.querySelector(`.${CSS.escape(name)}`);

function elementValue(element) {
  if (!element) return "";
  if (element.matches("input, textarea, select")) return element.value;
  return element.textContent ?? "";
}

function setElementText(name, value) {
  const element = namedElement(name);
  if (!element) {
    console.warn(`w3cp: element ".${name}" was not found`);
    return;
  }
  if (element.matches("input, textarea, select")) element.value = String(value);
  else element.textContent = String(value);
}

function getElementText(name) {
  const element = namedElement(name);
  if (!element) console.warn(`w3cp: element ".${name}" was not found`);
  return elementValue(element);
}

function currentPoints() {
  const screen = document.querySelector(".w3-screen.is-active");
  return [...(screen?.querySelectorAll("[data-w3-scroll-point]") ?? [])];
}

function scrollToPoint(name) {
  const target = namedElement(name);
  if (target) target.scrollIntoView({ behavior: "smooth", block: "start" });
  else console.warn(`w3cp: scroll point ".${name}" was not found`);
}

function scrollRelative(direction) {
  const points = currentPoints();
  if (!points.length) return;
  const y = window.scrollY + 8;
  let index = points.findIndex(point => point.getBoundingClientRect().top + window.scrollY >= y);
  if (index < 0) index = points.length - 1;
  if (direction < 0 && points[index].getBoundingClientRect().top + window.scrollY >= y) index--;
  else if (direction > 0 && points[index].getBoundingClientRect().top + window.scrollY < y) index++;
  points[Math.max(0, Math.min(points.length - 1, index))]?.scrollIntoView({
    behavior: "smooth",
    block: "start"
  });
}

function showScreen(name) {
  const target = document.querySelector(`.w3-screen[data-screen="${CSS.escape(name)}"]`);
  if (!target) {
    console.warn(`w3cp: screen "${name}" was not found`);
    return;
  }
  document.querySelectorAll(".w3-screen").forEach(screen => {
    screen.classList.toggle("is-active", screen === target);
  });
  document.title = target.dataset.title || document.title;
  window.scrollTo({ top: 0, behavior: "instant" });
}

function initializeTabs() {
  document.querySelectorAll("[data-w3-tabs]").forEach(tab => {
    const buttons = [...tab.querySelectorAll(":scope > .w3-tab-controls > [data-w3-tab-button]")];
    const pages = [...tab.querySelectorAll(":scope > .w3-tab-pages > [data-w3-tab-page]")];
    const select = index => {
      buttons.forEach((button, i) => {
        button.classList.toggle("is-active", i === index);
        button.setAttribute("aria-selected", String(i === index));
      });
      pages.forEach((page, i) => page.classList.toggle("is-active", i === index));
    };
    buttons.forEach((button, index) => button.addEventListener("click", () => select(index)));
    select(0);
  });
}

const functions = Object.create(null);
"#,
    );

    for section in sections {
        if matches!(section.kind, SectionKind::Function) {
            out.push_str(&format!(
                "\nfunctions[{}] = async function () {{\n{} }};\n",
                js_string(&section.name),
                render_actions(&section.actions, variables)?
            ));
        }
    }

    out.push_str(
        "\ndocument.addEventListener(\"DOMContentLoaded\", () => {\n  initializeTabs();\n",
    );
    out.push_str(&format!("  showScreen({});\n", js_string(start_screen)));
    for section in sections {
        if matches!(section.kind, SectionKind::Click) {
            out.push_str(&format!(
                "  document.querySelectorAll(`.${{CSS.escape({})}}`).forEach(element => {{\n    element.addEventListener(\"click\", async () => {{\n{}    }});\n  }});\n",
                js_string(&section.name),
                render_actions(&section.actions, variables)?
            ));
        }
    }
    out.push_str("});\n");
    Ok(out)
}

const CSS: &str = r#":root {
  color-scheme: light dark;
  --w3-accent: #0067c0;
  --w3-accent-hover: #1975c5;
  --w3-accent-pressed: #005a9e;
  --w3-bg: #f3f3f3;
  --w3-layer: rgba(255, 255, 255, .72);
  --w3-layer-solid: #fbfbfb;
  --w3-text: #1a1a1a;
  --w3-muted: #5d5d5d;
  --w3-stroke: rgba(0, 0, 0, .14);
  --w3-shadow: 0 2px 8px rgba(0, 0, 0, .12);
  font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; background: var(--w3-bg); color: var(--w3-text); }
body { margin: 0; min-height: 100vh; padding: 32px 28px 96px; background: radial-gradient(circle at 15% 0, rgba(0,103,192,.08), transparent 30rem), var(--w3-bg); }
button, input, textarea { font: inherit; }
.w3-screen { display: none; width: min(960px, 100%); margin: 0 auto; }
.w3-screen.is-active { display: block; animation: w3-enter .18s ease-out; }
@keyframes w3-enter { from { opacity: 0; transform: translateY(5px); } }
.w3-head { margin: 34px 0 14px; font-size: 28px; font-weight: 600; letter-spacing: -.02em; scroll-margin-top: 22px; }
.w3-text { line-height: 1.55; }
.w3-detail { color: var(--w3-muted); font-size: 13px; line-height: 1.45; }
.w3-button, .w3-tab-controls button {
  min-height: 34px; padding: 5px 14px; border: 1px solid var(--w3-stroke); border-bottom-color: rgba(0,0,0,.25);
  border-radius: 4px; color: var(--w3-text); background: var(--w3-layer-solid); box-shadow: 0 1px 1px rgba(0,0,0,.04);
  cursor: pointer; transition: background .1s, transform .1s, border-color .1s;
}
.w3-button:hover, .w3-tab-controls button:hover { background: #fff; }
.w3-button:active, .w3-tab-controls button:active { transform: scale(.98); background: #e9e9e9; }
.w3-button:focus-visible, input:focus-visible, textarea:focus-visible { outline: 2px solid var(--w3-accent); outline-offset: 2px; }
.w3-button-primary { color: white; background: var(--w3-accent); border-color: var(--w3-accent-pressed); }
.w3-button-primary:hover { background: var(--w3-accent-hover); }
.w3-button-text { background: transparent; border-color: transparent; box-shadow: none; }
.w3-card, .w3-tab, .w3-list-box {
  margin: 12px 0; padding: 18px; border: 1px solid var(--w3-stroke); border-radius: 8px; background: var(--w3-layer); box-shadow: var(--w3-shadow);
  backdrop-filter: blur(20px);
}
.w3-flex { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin: 10px 0; }
.w3-input, .w3-textarea {
  width: min(420px, 100%); min-height: 34px; padding: 6px 10px; color: var(--w3-text); background: var(--w3-layer-solid);
  border: 1px solid var(--w3-stroke); border-bottom: 2px solid #777; border-radius: 4px;
}
.w3-input:focus, .w3-textarea:focus { border-bottom-color: var(--w3-accent); outline: none; }
.w3-textarea { min-height: 100px; resize: vertical; }
.w3-toolbar {
  position: fixed; z-index: 100; left: 50%; bottom: 18px; transform: translateX(-50%); display: flex; align-items: center; gap: 8px;
  width: min(900px, calc(100% - 32px)); padding: 9px; border: 1px solid var(--w3-stroke); border-radius: 8px;
  background: rgba(250,250,250,.82); box-shadow: 0 8px 28px rgba(0,0,0,.16); backdrop-filter: blur(28px) saturate(1.4);
}
.w3-toolbar .w3-space { flex: 1; }
.w3-list-title { margin: 18px 0 8px; font-size: 20px; font-weight: 600; }
.w3-list-box { padding: 0; list-style: none; overflow: hidden; }
.w3-list { display: flex; justify-content: space-between; gap: 16px; padding: 12px 14px; border-bottom: 1px solid var(--w3-stroke); }
.w3-list:last-child { border-bottom: 0; }
.w3-code { overflow-x: auto; padding: 14px; border: 1px solid var(--w3-stroke); border-radius: 6px; background: #202020; color: #f6f6f6; font: 13px/1.55 "Cascadia Code", "SFMono-Regular", monospace; }
.w3-scroll-point { scroll-margin-top: 22px; }
.w3-tab { padding: 0; overflow: hidden; }
.w3-tab-controls { display: flex; gap: 3px; padding: 8px; border-bottom: 1px solid var(--w3-stroke); background: rgba(0,0,0,.025); }
.w3-tab-controls button { border-color: transparent; background: transparent; box-shadow: none; }
.w3-tab-controls button.is-active { color: var(--w3-accent); background: var(--w3-layer-solid); border-color: var(--w3-stroke); }
.w3-tab-page { display: none; padding: 18px; }
.w3-tab-page.is-active { display: block; animation: w3-enter .15s ease-out; }
.w3-tab-bottom { display: flex; flex-direction: column-reverse; }
.w3-tab-bottom > .w3-tab-controls { border-top: 1px solid var(--w3-stroke); border-bottom: 0; }
.w3-tab-left, .w3-tab-right { display: grid; grid-template-columns: auto 1fr; }
.w3-tab-right { grid-template-columns: 1fr auto; }
.w3-tab-right > .w3-tab-controls { grid-column: 2; grid-row: 1; }
.w3-tab-right > .w3-tab-pages { grid-column: 1; grid-row: 1; }
.w3-tab-left > .w3-tab-controls, .w3-tab-right > .w3-tab-controls { flex-direction: column; border-bottom: 0; border-right: 1px solid var(--w3-stroke); }
.w3-switch { display: inline-flex; margin: 8px 10px 8px 0; cursor: pointer; }
.w3-switch input { position: absolute; opacity: 0; pointer-events: none; }
.w3-switch span { width: 40px; height: 20px; padding: 2px; border: 1px solid #777; border-radius: 999px; background: transparent; transition: .15s; }
.w3-switch span::after { content: ""; display: block; width: 14px; height: 14px; border-radius: 50%; background: #666; transition: .15s; }
.w3-switch input:checked + span { border-color: var(--w3-accent); background: var(--w3-accent); }
.w3-switch input:checked + span::after { transform: translateX(20px); background: white; }
@media (prefers-color-scheme: dark) {
  :root { --w3-bg: #202020; --w3-layer: rgba(45,45,45,.74); --w3-layer-solid: #333; --w3-text: #fff; --w3-muted: #c7c7c7; --w3-stroke: rgba(255,255,255,.12); --w3-shadow: 0 2px 8px rgba(0,0,0,.4); }
  .w3-button:hover, .w3-tab-controls button:hover { background: #3b3b3b; }
  .w3-toolbar { background: rgba(38,38,38,.84); }
}
@media (max-width: 600px) {
  body { padding: 22px 16px 90px; }
  .w3-toolbar { bottom: 10px; }
  .w3-head { font-size: 24px; }
}
"#;

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let source_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let output_dir = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| source_dir.join("dist"));
    let config_path = source_dir.join("config.ini");
    let config_source = fs::read_to_string(&config_path)
        .map_err(|error| format!("{} を読めません: {error}", config_path.display()))?;
    let config = ini(&config_source);
    if config.get("version").map(String::as_str) != Some("3") {
        return Err("config.ini の `version=3` が必要です".into());
    }
    let start_screen = config
        .get("screen")
        .cloned()
        .unwrap_or_else(|| "main".into());
    let app_name = config
        .get("name")
        .cloned()
        .unwrap_or_else(|| "Warp 3 App".into());

    let mut ui_files: Vec<PathBuf> = fs::read_dir(&source_dir)
        .map_err(|error| format!("{} を読めません: {error}", source_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("w2u"))
        .collect();
    ui_files.sort();
    if ui_files.is_empty() {
        return Err("`.w2u` ファイルが見つかりません".into());
    }

    let mut screens = Vec::new();
    let mut script_names = BTreeSet::new();
    for path in ui_files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("{} を読めません: {error}", path.display()))?;
        let nodes = UiParser::new(&source)
            .parse()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let screen = path.file_stem().unwrap().to_string_lossy().to_string();
        let mut title = None;
        collect_config(&nodes, &mut title, &mut script_names);
        screens.push((screen, title.unwrap_or_else(|| app_name.clone()), nodes));
    }

    let mut sections = Vec::new();
    for script in script_names {
        let path = source_dir.join(&script);
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("{} を読めません: {error}", path.display()))?;
        sections.extend(parse_script(&source, &path)?);
    }
    let variables = variable_names(&sections);
    let js = generate_js(&sections, &variables, &start_screen)?;

    let mut body = String::new();
    for (screen, title, nodes) in &screens {
        body.push_str(&format!(
            "<main class=\"w3-screen\" data-screen=\"{}\" data-title=\"{}\">",
            escape_html(screen),
            escape_html(title)
        ));
        render_children(nodes, &mut body);
        body.push_str("</main>");
    }
    let html = format!(
        "<!doctype html>\n<html lang=\"ja\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>{}</title>\n<link rel=\"stylesheet\" href=\"style.css\">\n<script src=\"app.js\" defer></script>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        escape_html(&app_name),
        body
    );

    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("{} を作成できません: {error}", output_dir.display()))?;
    fs::write(output_dir.join("index.html"), html)
        .map_err(|error| format!("index.html を書けません: {error}"))?;
    fs::write(output_dir.join("app.js"), js)
        .map_err(|error| format!("app.js を書けません: {error}"))?;
    fs::write(output_dir.join("style.css"), CSS)
        .map_err(|error| format!("style.css を書けません: {error}"))?;
    println!(
        "w3cp: {} screen(s) -> {}",
        screens.len(),
        output_dir.display()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("w3cp error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_ui_and_escapes_text() {
        let nodes = UiParser::new(
            r#"head,scroll-point.title { text("Hello\n<world>") }
               button.go { text("Go") type("primary") }"#,
        )
        .parse()
        .unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].tags, vec!["head", "scroll-point"]);
        let mut html = String::new();
        render_children(&nodes, &mut html);
        assert!(html.contains("data-w3-scroll-point"));
        assert!(html.contains("Hello<br>&lt;world&gt;"));
    }

    #[test]
    fn parses_script_sections() {
        let sections = parse_script(
            "[onClick = add]\ncount = +1\nfun = update\n",
            Path::new("test.w2s"),
        )
        .unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].actions.len(), 2);
    }
}
