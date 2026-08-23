use roxmltree::Document;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[path = "../rust/ui/main.rs"]
mod ui;

#[derive(Clone, Debug)]
enum ScriptNode {
    Raw(String),
    Block { header: String, body: Vec<ScriptNode> },
}

#[derive(Clone, Debug)]
enum Op {
    Command { name: String, target: Option<String>, value: Option<String> },
    If { left: String, operator: String, right: String, body: Vec<Op> },
}

#[derive(Default)]
struct ScriptProgram {
    init: Vec<Op>,
    events: BTreeMap<String, Vec<Op>>,
    functions: BTreeMap<String, Vec<Op>>,
}

struct Screen {
    name: String,
    body: String,
    script: ScriptProgram,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("warp4-example: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_else(|| "warp4-example".into());
    let input_dir = args.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("example/src"));
    let output_dir = args.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("example/output"));
    if args.next().is_some() {
        return Err(format!("使い方: {} [入力ディレクトリ] [出力ディレクトリ]", PathBuf::from(program).display()).into());
    }
    if !input_dir.is_dir() {
        return Err(format!("入力ディレクトリがありません: {}", input_dir.display()).into());
    }

    let config = read_config(&input_dir.join("config.ini"))?;
    let entry = config.get("screen").cloned().unwrap_or_else(|| "main".into());
    let mut layouts = BTreeMap::new();
    for path in sorted_files(&input_dir, &["w3u", "w4u"])? {
        let name = path.file_stem().and_then(|x| x.to_str()).ok_or("画面名を取得できません")?;
        let xml = fs::read_to_string(&path)?;
        let document = Document::parse(&xml).map_err(|e| format!("XML parse error in {}: {e}", path.display()))?;
        layouts.insert(name.to_string(), ui::render_node(document.root_element(), "", "")?);
    }
    if layouts.is_empty() { return Err(format!("{}.w3u/.w4u が見つかりません", input_dir.display()).into()); }
    if !layouts.contains_key(&entry) { return Err(format!("config.ini の screen '{}' に対応する画面がありません", entry).into()); }

    let mut scripts = BTreeMap::new();
    for path in sorted_files(&input_dir, &["w3s", "w4s"])? {
        let name = path.file_stem().and_then(|x| x.to_str()).ok_or("スクリプト名を取得できません")?;
        let source = strip_comments(&fs::read_to_string(&path)?);
        scripts.insert(name.to_string(), compile_script(&source).map_err(|e| format!("{}: {e}", path.display()))?);
    }

    let screens = layouts.into_iter().map(|(name, body)| Screen {
        script: scripts.remove(&name).unwrap_or_default(),
        name,
        body,
    }).collect::<Vec<_>>();
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("index.html");
    fs::write(&output_path, build_document(&entry, &config, &screens))?;
    println!("{}", output_path.display());
    Ok(())
}

fn sorted_files(dir: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = fs::read_dir(dir)?.filter_map(|x| x.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()).is_some_and(|x| extensions.contains(&x)))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn read_config(path: &Path) -> Result<HashMap<String, String>, Box<dyn Error>> {
    if !path.is_file() { return Ok(HashMap::new()); }
    let mut values = HashMap::new();
    for (line_no, line) in fs::read_to_string(path)?.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') { continue; }
        let Some((key, value)) = line.split_once('=') else { return Err(format!("{}:{}: key=value の形式が必要です", path.display(), line_no + 1).into()); };
        values.insert(key.trim().into(), value.trim().into());
    }
    Ok(values)
}

fn compile_script(source: &str) -> Result<ScriptProgram, String> {
    let mut parser = ScriptParser::new(source);
    let nodes = parser.parse_program(false)?;
    let mut program = ScriptProgram::default();
    for node in nodes {
        match node {
            ScriptNode::Raw(raw) => program.init.push(compile_raw(&raw)?),
            ScriptNode::Block { header, body } => {
                let header = header.trim();
                if let Some(name) = parse_fun_definition(header)? {
                    if program.functions.insert(name.clone(), compile_ops(&body)?).is_some() {
                        return Err(format!("関数 '{}' は重複定義できません", name));
                    }
                } else if let Some(target) = header.strip_prefix("WarpUI.OnClick") {
                    let target = target.trim();
                    if target.is_empty() { return Err("WarpUI.OnClick のIDが空です".into()); }
                    program.events.insert(target.into(), compile_ops(&body)?);
                } else {
                    return Err(format!("トップレベルで対応していないブロックです: {header}"));
                }
            }
        }
    }
    Ok(program)
}

fn compile_ops(nodes: &[ScriptNode]) -> Result<Vec<Op>, String> {
    nodes.iter().map(|node| match node {
        ScriptNode::Raw(raw) => compile_raw(raw),
        ScriptNode::Block { header, body } => {
            let condition = header.strip_prefix("if").filter(|_| keyword_boundary(header, "if"))
                .ok_or_else(|| format!("対応していないブロックです: {header}"))?;
            let (left, operator, right) = split_condition(condition.trim())
                .ok_or_else(|| format!("if の条件を解析できません: {header}"))?;
            Ok(Op::If { left, operator, right, body: compile_ops(body)? })
        }
    }).collect()
}

fn compile_raw(raw: &str) -> Result<Op, String> {
    let stmt = raw.trim();
    if stmt.is_empty() { return Err("空の命令です".into()); }
    if stmt == "break" { return Ok(command("break", None, None)); }
    if let Some(inline) = parse_inline_if(stmt)? { return Ok(inline); }

    let (name, rest) = split_command(stmt);
    if name.is_empty() { return Err(format!("命令を解析できません: {stmt}")); }
    let (target, value) = if rest.trim().is_empty() {
        (None, None)
    } else if rest.trim_start().starts_with('(') {
        let (value, end) = read_balanced_str(rest.trim_start(), 0, '(', ')')?;
        if !rest.trim_start()[end..].trim().is_empty() { return Err(format!("命令の後ろに文字があります: {stmt}")); }
        (None, Some(value.to_string()))
    } else if matches!(name, "var.set" | "var.edit" | "const.set") || name.starts_with("WarpUI.") {
        let (target, value) = parse_target_value(rest.trim_start())?;
        (Some(target), Some(value))
    } else {
        (None, Some(rest.trim().to_string()))
    };
    Ok(command(name, target, value))
}

fn command(name: &str, target: Option<String>, value: Option<String>) -> Op {
    Op::Command { name: name.into(), target, value }
}

fn split_command(stmt: &str) -> (&str, &str) {
    let end = stmt.find(|c: char| c.is_whitespace() || c == '(').unwrap_or(stmt.len());
    (&stmt[..end], &stmt[end..])
}

fn parse_target_value(rest: &str) -> Result<(String, String), String> {
    let open = find_top_level_char(rest, '(').ok_or("命令は '名前 (値)' の形式で指定してください")?;
    let target = rest[..open].trim();
    if target.is_empty() { return Err("命令の名前またはIDが空です".into()); }
    let (value, end) = read_balanced_str(rest, open, '(', ')')?;
    if !rest[end..].trim().is_empty() { return Err("値の後ろに解釈できない文字があります".into()); }
    Ok((target.into(), value.into()))
}

fn parse_fun_definition(header: &str) -> Result<Option<String>, String> {
    if !keyword_boundary(header, "fun") { return Ok(None); }
    let rest = header[3..].trim_start();
    if !rest.starts_with('(') { return Ok(None); }
    let (inside, end) = read_balanced_str(rest, 0, '(', ')')?;
    if !rest[end..].trim().is_empty() { return Ok(None); }
    if inside.trim().is_empty() { return Err("fun の関数名が空です".into()); }
    Ok(Some(inside.trim().into()))
}

fn parse_inline_if(stmt: &str) -> Result<Option<Op>, String> {
    if !keyword_boundary(stmt, "if") { return Ok(None); }
    let rest = stmt[2..].trim_start();
    let chars = rest.chars().collect::<Vec<_>>();
    let mut bracket = 0i32;
    let mut open = None;
    for (index, c) in chars.iter().enumerate() {
        match c {
            '[' => bracket += 1,
            ']' => bracket -= 1,
            '(' if bracket == 0 => { open = Some(index); break; }
            _ => {}
        }
    }
    let Some(open) = open else { return Ok(None); };
    let condition = chars[..open].iter().collect::<String>();
    let Some((left, operator, right)) = split_condition(condition.trim()) else { return Ok(None); };
    let byte_open = rest.char_indices().nth(open).map(|x| x.0).unwrap_or(rest.len());
    let (body, end) = read_balanced_str(rest, byte_open, '(', ')')?;
    if !rest[end..].trim().is_empty() { return Ok(None); }
    let mut parser = ScriptParser::new(body);
    Ok(Some(Op::If { left, operator, right, body: compile_ops(&parser.parse_program(false)?)? }))
}

fn keyword_boundary(value: &str, keyword: &str) -> bool {
    value.strip_prefix(keyword).is_some_and(|rest| rest.is_empty() || rest.chars().next().is_some_and(char::is_whitespace) || rest.starts_with('('))
}

fn split_condition(value: &str) -> Option<(String, String, String)> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut square = 0i32;
    let mut paren = 0i32;
    let mut i = 0;
    while i < chars.len() {
        let found = if square == 0 && paren == 0 && i + 1 < chars.len() && chars[i] == '!' && chars[i + 1] == '=' { Some(("!=", 2)) }
        else if square == 0 && paren == 0 && chars[i] == '=' { Some(("=", 1)) }
        else if square == 0 && paren == 0 && chars[i] == '<' { Some(("<", 1)) }
        else if square == 0 && paren == 0 && chars[i] == '>' { Some((">", 1)) } else { None };
        if let Some((operator, width)) = found {
            return Some((chars[..i].iter().collect::<String>().trim().into(), operator.into(), chars[i + width..].iter().collect::<String>().trim().into()));
        }
        match chars[i] { '[' => square += 1, ']' => square -= 1, '(' => paren += 1, ')' => paren -= 1, _ => {} }
        i += 1;
    }
    None
}

struct ScriptParser { chars: Vec<char>, pos: usize }
impl ScriptParser {
    fn new(source: &str) -> Self { Self { chars: source.chars().collect(), pos: 0 } }
    fn parse_program(&mut self, stop_on_brace: bool) -> Result<Vec<ScriptNode>, String> {
        let mut nodes = Vec::new();
        while !self.eof() {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) { self.pos += 1; }
            if self.eof() { break; }
            let start = self.pos;
            let mut square = 0i32;
            let mut paren = 0i32;
            let mut consumed = false;
            while !self.eof() {
                let c = self.chars[self.pos];
                match c {
                    '[' => square += 1,
                    ']' => square -= 1,
                    '(' => paren += 1,
                    ')' => paren -= 1,
                    '{' if square == 0 && paren == 0 => {
                        let header = self.chars[start..self.pos].iter().collect::<String>().trim().to_string();
                        self.pos += 1;
                        nodes.push(ScriptNode::Block { header, body: self.parse_program(true)? });
                        consumed = true;
                        break;
                    }
                    '\n' if square == 0 && paren == 0 => {
                        let raw = self.chars[start..self.pos].iter().collect::<String>().trim().to_string();
                        self.pos += 1;
                        if !raw.is_empty() { nodes.push(ScriptNode::Raw(raw)); }
                        consumed = true;
                        break;
                    }
                    '}' if square == 0 && paren == 0 => {
                        if !stop_on_brace { return Err("対応する '{' がない '}' があります".into()); }
                        let raw = self.chars[start..self.pos].iter().collect::<String>().trim().to_string();
                        if !raw.is_empty() { nodes.push(ScriptNode::Raw(raw)); }
                        self.pos += 1;
                        return Ok(nodes);
                    }
                    _ => {}
                }
                self.pos += 1;
            }
            if !consumed && self.eof() {
                let raw = self.chars[start..self.pos].iter().collect::<String>().trim().to_string();
                if !raw.is_empty() { nodes.push(ScriptNode::Raw(raw)); }
            }
        }
        if stop_on_brace { return Err("'{' に対応する '}' がありません".into()); }
        Ok(nodes)
    }
    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn eof(&self) -> bool { self.pos >= self.chars.len() }
}

fn find_top_level_char(value: &str, wanted: char) -> Option<usize> {
    let mut square = 0i32;
    for (index, c) in value.char_indices() {
        if c == wanted && square == 0 { return Some(index); }
        match c { '[' => square += 1, ']' => square -= 1, _ => {} }
    }
    None
}

fn read_balanced_str(value: &str, start: usize, open: char, close: char) -> Result<(&str, usize), String> {
    if value.get(start..).and_then(|x| x.chars().next()) != Some(open) { return Err(format!("'{}' が必要です", open)); }
    let mut depth = 0i32;
    let mut inner_start = None;
    for (relative, c) in value[start..].char_indices() {
        let index = start + relative;
        if c == open { depth += 1; if depth == 1 { inner_start = Some(index + c.len_utf8()); } }
        else if c == close { depth -= 1; if depth == 0 { return Ok((&value[inner_start.ok_or("parser state")?..index], index + c.len_utf8())); } }
    }
    Err(format!("'{}' が閉じられていません", open))
}

fn strip_comments(source: &str) -> String {
    source.lines().map(|line| line.split_once('#').map_or(line, |(before, _)| before)).collect::<Vec<_>>().join("\n")
}

fn js_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029"))
}

fn op_js(op: &Op) -> String {
    match op {
        Op::Command { name, target, value } => format!("{{kind:\"command\",name:{},target:{},value:{}}}", js_string(name), target.as_deref().map(js_string).unwrap_or_else(|| "null".into()), value.as_deref().map(js_string).unwrap_or_else(|| "null".into())),
        Op::If { left, operator, right, body } => format!("{{kind:\"if\",left:{},operator:{},right:{},body:[{}]}}", js_string(left), js_string(operator), js_string(right), ops_js(body)),
    }
}
fn ops_js(ops: &[Op]) -> String { ops.iter().map(op_js).collect::<Vec<_>>().join(",") }
fn script_js(script: &ScriptProgram) -> String {
    let events = script.events.iter().map(|(id, ops)| format!("{}:[{}]", js_string(id), ops_js(ops))).collect::<Vec<_>>().join(",");
    let functions = script.functions.iter().map(|(name, ops)| format!("{{name:{},body:[{}]}}", js_string(name), ops_js(ops))).collect::<Vec<_>>().join(",");
    format!("{{init:[{}],events:{{{events}}},functions:[{functions}]}}", ops_js(&script.init))
}

fn build_document(entry: &str, config: &HashMap<String, String>, screens: &[Screen]) -> String {
    let title = config.get("name").map(String::as_str).unwrap_or("Warp 4 Example");
    let bodies = screens.iter().map(|screen| format!("<section class=\"warp-screen\" id=\"screen-{}\" data-screen=\"{}\">{}</section>", esc_attr(&screen.name), esc_attr(&screen.name), screen.body)).collect::<String>();
    let data = screens.iter().map(|screen| format!("{}:{}", js_string(&screen.name), script_js(&screen.script))).collect::<Vec<_>>().join(",");
    let css = reference_css();
    format!(r#"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<title>{}</title>
<style>{}
.warp-screen{{display:none;width:100%;height:100%;overflow:hidden}}
.warp-screen.active{{display:block}}
</style>
</head>
<body><div id="app">{}</div>
<script>
const SCREENS={{{}}};
const ENTRY={};
{}
</script>
</body>
</html>
"#, esc_text(title), css, bodies, data, js_string(entry), format!("{}\n{}", ui::RUNTIME_JS, RUNTIME_JS))
}

const HTML_REFERENCE: &str = include_str!("../html/index.html");

fn reference_css() -> &'static str {
    HTML_REFERENCE.split_once("<style>")
        .and_then(|(_, rest)| rest.split_once("</style>").map(|(css, _)| css))
        .unwrap_or(ui::CSS)
}

fn esc_text(value: &str) -> String { value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }
fn esc_attr(value: &str) -> String { esc_text(value).replace('"', "&quot;") }

const RUNTIME_JS: &str = r###"
"use strict";
const scriptState={vars:Object.create(null),consts:Object.create(null)};
const scriptFunctions=Object.create(null);
const SCRIPT_BREAK=Symbol("break");
const SCRIPT_NONE=Symbol("none");
let scriptCurrentScreen="";

function scriptFindSquare(value,open){let depth=0;for(let i=open;i<value.length;i++){if(value[i]==="[")depth++;else if(value[i]==="]"){depth--;if(depth===0)return i}}return -1}
function scriptCalc(value){const expr=String(value);if(!/^[0-9+*/%().\s-]+$/.test(expr))throw new Error("calc の式を解析できません: "+expr);const result=Function("return ("+expr+")")();if(!Number.isFinite(result))throw new Error("calc の結果が有限の数値ではありません");return Number.isInteger(result)?String(result):String(result)}
function scriptExpandPass(input){let out="",changed=false;for(let i=0;i<input.length;){let kind=input.startsWith("var[",i)?"var":input.startsWith("const[",i)?"const":input.startsWith("calc[",i)?"calc":null;if(!kind){out+=input[i++];continue}const open=i+kind.length,close=scriptFindSquare(input,open);if(close<0)throw new Error(kind+"[ ... ] の ']' が不足しています");const inner=scriptExpandAll(input.slice(open+1,close));if(kind==="calc"){out+=scriptCalc(inner);changed=true}else{const table=kind==="var"?scriptState.vars:scriptState.consts;const key=inner.trim();if(Object.prototype.hasOwnProperty.call(table,key)){out+=table[key];changed=true}else out+=input.slice(i,close+1)}i=close+1}return [out,changed]}
function scriptExpandAll(input){let current=String(input??"");for(let i=0;i<256;i++){const [next,changed]=scriptExpandPass(current);if(!changed||next===current)return next;current=next}throw new Error("展開回数が上限を超えました。循環参照の可能性があります")}
function scriptCompare(left,operator,right){let a=scriptExpandAll(left).trim(),b=scriptExpandAll(right).trim();if(a==="()")a="";if(b==="()")b="";if(operator==="=")return a===b;if(operator==="!=")return a!==b;const x=Number(a),y=Number(b);if(!Number.isFinite(x)||!Number.isFinite(y))return false;return operator==="<"?x<y:x>y}
function scriptFind(id){const root=document.querySelector(".warp-screen.active");if(!root)return null;const wanted=scriptExpandAll(id).trim();return q("[data-baram-id]",root).find(el=>el.dataset.baramId===wanted)||null}
function scriptDim(value){const v=String(value??"");if(v==="match_parent"||v==="fill_parent")return "100%";if(v==="wrap_content")return "auto";if(v==="0dp")return "0px";if(/^[-+]?(?:\d+\.?\d*|\.\d+)(?:dp|dip|sp|px)?$/.test(v))return parseFloat(v)+"px";return v}
function scriptSetProperty(command,id,value){const el=scriptFind(id);if(!el)return;const v=scriptExpandAll(value).trim();const css={textColor:"color",background:"background",textSize:"fontSize",alpha:"opacity",layout_width:"width",layout_height:"height"}[command];if(css)el.style[css]=css==="fontSize"||command.startsWith("layout_")?scriptDim(v):v;else if(command==="layout_weight"){el.style.flexGrow=Number(v)||0}else if(command.startsWith("padding")||command.startsWith("layout_margin")){const prop=command.replace(/[A-Z]/g,m=>"-"+m.toLowerCase()).replace("layout-","");el.style[prop]=scriptDim(v)}else if(command.startsWith("layout_")||command==="gravity"){el.setAttribute("data-w3u-"+command.toLowerCase(),v)}if(typeof relayout==="function")requestAnimationFrame(relayout)}
const scriptUi={
 text:(id,value)=>{const el=scriptFind(id);if(!el)return;const text=scriptExpandAll(value);if(el.matches("input,textarea,select"))el.value=text;else if(el.classList.contains("baram-switch")){const label=el.querySelector(".control-text");if(label)label.textContent=text}else el.textContent=text},
 getText:id=>{const el=scriptFind(id);return el?.value??el?.textContent??""},
 editText:(id,value)=>{const el=scriptFind(id);if(el&&el.matches("input,textarea"))el.value=scriptExpandAll(value)},
 visibility:(id,value)=>{const el=scriptFind(id);if(!el)return;const v=scriptExpandAll(value).trim();el.style.display=v==="gone"?"none":"";el.style.visibility=v==="invisible"?"hidden":""},
 screen:name=>scriptActivateScreen(scriptExpandAll(name).replace(/\.(?:w3u|w4u)$/,""))
};
function scriptDuration(value){const match=String(value).trim().match(/^([0-9]+(?:\.[0-9]+)?)(ns|us|ms|s|m|h)$/);if(!match)throw new Error("wait の時間に単位がありません: "+value);const n=Number(match[1]),unit=match[2];return n*(unit==="ns"?1e-6:unit==="us"?1e-3:unit==="ms"?1:unit==="s"?1000:unit==="m"?60000:3600000)}
function scriptDelay(value){return new Promise(resolve=>setTimeout(resolve,scriptDuration(value)))}
async function scriptCallFunction(name,loopDepth){const key=scriptExpandAll(name).trim(),body=scriptFunctions[key];if(!body)throw new Error("関数 '"+key+"' は定義されていません");return scriptRunOps(body,loopDepth)}
async function scriptExecuteCommand(op,loopDepth){
 const command=scriptExpandAll(op.name).trim(),target=op.target==null?null:scriptExpandAll(op.target).trim(),value=op.value==null?"":scriptExpandAll(op.value);
 if(command==="var.set"){if(!(target in scriptState.vars))scriptState.vars[target]=value;return SCRIPT_NONE}
 if(command==="var.edit"){if(target in scriptState.vars)scriptState.vars[target]=value;return SCRIPT_NONE}
 if(command==="const.set"){if(!(target in scriptState.consts))scriptState.consts[target]=value;return SCRIPT_NONE}
 if(command==="break")return loopDepth>0?SCRIPT_BREAK:SCRIPT_NONE;
 if(command==="print"){console.log(value);return SCRIPT_NONE}
 if(command==="wait"){await scriptDelay(value);return SCRIPT_NONE}
 if(command==="fun")return scriptCallFunction(value,loopDepth);
 if(command==="for"){for(;;){const signal=await scriptCallFunction(value,loopDepth+1);if(signal===SCRIPT_BREAK)break;if(signal!==SCRIPT_NONE)return signal}return SCRIPT_NONE}
 if(command==="WarpUI.screen"){await scriptUi.screen(value);return SCRIPT_NONE}
 if(command.startsWith("WarpUI.")){
  const action=command.slice("WarpUI.".length);
  if(action==="text")scriptUi.text(target,value);
  else if(action==="textColor"||action==="background"||action==="textSize")scriptSetProperty(action,target,value);
  else if(action==="visibility")scriptUi.visibility(target,value);
  else if(action==="editText")scriptUi.editText(target,value);
  else if(action==="getText"){if(value in scriptState.vars)scriptState.vars[value]=scriptUi.getText(target)}
  else if(action!=="OnClick")scriptSetProperty(action,target,value);
  return SCRIPT_NONE;
 }
 throw new Error("不明な命令です: "+command);
}
async function scriptRunOps(ops,loopDepth=0){for(const op of ops||[]){let signal=SCRIPT_NONE;if(op.kind==="command")signal=await scriptExecuteCommand(op,loopDepth);else if(op.kind==="if"&&scriptCompare(op.left,op.operator,op.right))signal=await scriptRunOps(op.body,loopDepth);if(signal!==SCRIPT_NONE)return signal}return SCRIPT_NONE}
function scriptRegisterFunctions(program){for(const definition of program?.functions||[]){const name=scriptExpandAll(definition.name).trim();if(name)scriptFunctions[name]=definition.body}}
function scriptWire(screen){const program=SCREENS[screen];if(!program)return;for(const [id,ops] of Object.entries(program.events||{})){const el=scriptFind(id);if(!el||el.dataset.scriptWired)return;el.dataset.scriptWired="true";el.addEventListener("click",async event=>{event.stopPropagation();try{await scriptRunOps(ops)}catch(error){console.error(error)}})}}
async function scriptActivateScreen(name){const next=String(name);const screen=document.getElementById("screen-"+CSS.escape(next));if(!screen)return;q(".warp-screen").forEach(el=>el.classList.toggle("active",el===screen));scriptCurrentScreen=next;const program=SCREENS[next];scriptRegisterFunctions(program);try{await scriptRunOps(program.init)}catch(error){console.error(error)}scriptWire(next);if(typeof relayout==="function")requestAnimationFrame(relayout)}
scriptRegisterFunctions(SCREENS[ENTRY]);
scriptActivateScreen(ENTRY);
"###;
