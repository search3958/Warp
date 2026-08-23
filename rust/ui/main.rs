use roxmltree::{Document, Node};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

const OUTPUT_FILE: &str = "w3u-output.html";

fn main() {
    if let Err(err) = run() {
        eprintln!("w3u-parser: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|s| PathBuf::from(s).file_name().map(|x| x.to_owned()))
        .unwrap_or_else(|| "w3u-parser".into());

    let input = match args.next() {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!("Usage: {} <layout.xml>", PathBuf::from(program).display());
            return Err("input XML file is required".into());
        }
    };

    if args.next().is_some() {
        return Err("only one input file can be specified".into());
    }

    if !input.is_file() {
        return Err(format!("input file not found: {}", input.display()).into());
    }

    let xml = fs::read_to_string(&input)?;
    let document = Document::parse(&xml)
        .map_err(|e| format!("XML parse error in {}: {e}", input.display()))?;

    let root = document.root_element();
    let body = render_node(root, "", "")?;
    let output = build_document(&body);

    let output_path = input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(OUTPUT_FILE);

    fs::write(&output_path, output)?;
    println!("{}", output_path.display());
    Ok(())
}

pub fn render_node(node: Node<'_, '_>, parent_tag: &str, parent_orientation: &str) -> Result<String, Box<dyn Error>> {
    let tag = node.tag_name().name();

    if tag == "TableLayout" {
        return render_table(node, parent_tag, parent_orientation);
    }

    let orientation = if matches!(tag, "LinearLayout" | "RadioGroup") {
        match attr(node, "orientation") {
            Some("horizontal") => "horizontal",
            _ => "vertical",
        }
    } else {
        ""
    };

    let mut common = common_parts(node, parent_tag, parent_orientation);
    let text = esc_text(attr(node, "text").unwrap_or(""));
    let hint = esc_attr(attr(node, "hint").unwrap_or(""));

    let html = match tag {
        "LinearLayout" => {
            common.classes.push("baram-linear".into());
            common.styles.push(format!(
                "flex-direction:{}",
                if orientation == "horizontal" { "row" } else { "column" }
            ));
            common.attrs.push(format!("data-orientation=\"{}\"", orientation));
            wrap("div", &common, &render_children(node, tag, orientation)?)
        }
        "RelativeLayout" => {
            common.classes.push("baram-relative".into());
            wrap("div", &common, &render_children(node, tag, "")?)
        }
        "FrameLayout" => {
            common.classes.push("baram-frame".into());
            wrap("div", &common, &render_children(node, tag, "")?)
        }
        "GridLayout" => {
            common.classes.push("baram-grid".into());
            let cols = number(attr(node, "columnCount"), 1.0).max(1.0) as i64;
            common.styles.push(format!("grid-template-columns:repeat({cols},minmax(0,1fr))"));
            common.styles.push("gap:8px".into());
            wrap("div", &common, &render_children(node, tag, "")?)
        }
        "AbsoluteLayout" => {
            common.classes.push("baram-absolute".into());
            wrap("div", &common, &render_children(node, tag, "")?)
        }
        "ScrollView" => {
            common.classes.push("baram-scroll".into());
            common.styles.push("overflow-y:auto".into());
            common.styles.push("overflow-x:hidden".into());
            wrap("div", &common, &render_children(node, tag, "")?)
        }
        "HorizontalScrollView" => {
            common.classes.push("baram-scroll".into());
            common.styles.push("overflow-x:auto".into());
            common.styles.push("overflow-y:hidden".into());
            wrap("div", &common, &render_children(node, tag, "")?)
        }
        "RadioGroup" => {
            common.classes.push("baram-linear".into());
            common.classes.push("baram-radio-group".into());
            common.styles.push(format!(
                "flex-direction:{}",
                if orientation == "horizontal" { "row" } else { "column" }
            ));
            common.attrs.push(format!("data-orientation=\"{}\"", orientation));
            wrap("div", &common, &render_children(node, tag, orientation)?)
        }
        "TextView" => {
            common.classes.push("baram-text".into());
            apply_text(node, &mut common);
            wrap("div", &common, &text)
        }
        "EditText" | "AutoCompleteTextView" | "MultiAutoCompleteTextView" => {
            common.classes.push("baram-edit".into());
            apply_text(node, &mut common);
            let multi = tag == "MultiAutoCompleteTextView" || attr(node, "singleLine") == Some("false");
            if multi {
                let mut attrs = common;
                if !hint.is_empty() {
                    attrs.attrs.push(format!("placeholder=\"{}\"", hint));
                }
                wrap("textarea", &attrs, &text)
            } else {
                let mut attrs = common;
                attrs.attrs.push("type=\"text\"".into());
                if !hint.is_empty() {
                    attrs.attrs.push(format!("placeholder=\"{}\"", hint));
                }
                if !text.is_empty() {
                    attrs.attrs.push(format!("value=\"{}\"", esc_attr(attr(node, "text").unwrap_or(""))));
                }
                void_tag("input", &attrs)
            }
        }
        "Button" | "PrimaryButton" => {
            common.classes.push(if tag == "PrimaryButton" {
                "baram-primary-button".into()
            } else {
                "baram-button".into()
            });
            common.attrs.push("type=\"button\"".into());
            apply_text(node, &mut common);
            wrap("button", &common, if text.is_empty() { "Button" } else { &text })
        }
        "ImageButton" => {
            common.classes.push("baram-image-button".into());
            common.attrs.push("type=\"button\"".into());
            common.attrs.push(format!(
                "aria-label=\"{}\"",
                esc_attr(attr(node, "contentDescription").unwrap_or("ImageButton"))
            ));
            wrap("button", &common, "<span class=\"camera-icon\" aria-hidden=\"true\"></span>")
        }
        "CheckBox" => {
            common.classes.push("baram-checkbox".into());
            common.attrs.push("tabindex=\"0\"".into());
            common.attrs.push("role=\"checkbox\"".into());
            common.attrs.push(format!(
                "data-checked=\"{}\"",
                bool_attr(node, "checked", false)
            ));
            let label = if text.is_empty() { "CheckBox" } else { &text };
            wrap(
                "div",
                &common,
                &format!("<span class=\"mark\"></span><span class=\"control-text\">{label}</span>"),
            )
        }
        "RadioButton" => {
            common.classes.push("baram-radio".into());
            common.attrs.push("tabindex=\"0\"".into());
            common.attrs.push("role=\"radio\"".into());
            common.attrs.push(format!(
                "data-checked=\"{}\"",
                bool_attr(node, "checked", false)
            ));
            let label = if text.is_empty() { "RadioButton" } else { &text };
            wrap(
                "div",
                &common,
                &format!("<span class=\"mark\"></span><span class=\"control-text\">{label}</span>"),
            )
        }
        "ToggleButton" => {
            common.classes.push("baram-toggle".into());
            common.attrs.push("type=\"button\"".into());
            common.attrs.push(format!(
                "data-checked=\"{}\"",
                bool_attr(node, "checked", false)
            ));
            common.attrs.push(format!(
                "data-text-on=\"{}\"",
                esc_attr(attr(node, "textOn").unwrap_or("ON"))
            ));
            common.attrs.push(format!(
                "data-text-off=\"{}\"",
                esc_attr(attr(node, "textOff").unwrap_or("OFF"))
            ));
            let checked = bool_attr(node, "checked", false);
            let label = if checked {
                esc_text(attr(node, "textOn").unwrap_or("ON"))
            } else {
                esc_text(attr(node, "textOff").unwrap_or("OFF"))
            };
            wrap("button", &common, &label)
        }
        "Switch" => {
            common.classes.push("baram-switch".into());
            common.attrs.push("tabindex=\"0\"".into());
            common.attrs.push("role=\"switch\"".into());
            common.attrs.push(format!(
                "data-checked=\"{}\"",
                bool_attr(node, "checked", false)
            ));
            let label = if text.is_empty() { "Switch" } else { &text };
            wrap(
                "div",
                &common,
                &format!("<span class=\"track\"><span class=\"thumb\"></span></span><span class=\"control-text\">{label}</span>"),
            )
        }
        "Spinner" => {
            common.classes.push("baram-spinner".into());
            wrap(
                "select",
                &common,
                "<option>Item 1</option><option>Item 2</option><option>Item 3</option>",
            )
        }
        "SeekBar" => {
            common.classes.push("baram-seek".into());
            common.attrs.push("type=\"range\"".into());
            common.attrs.push(format!("min=\"{}\"", esc_attr(attr(node, "min").unwrap_or("0"))));
            common.attrs.push(format!("max=\"{}\"", esc_attr(attr(node, "max").unwrap_or("100"))));
            common.attrs.push(format!("value=\"{}\"", esc_attr(attr(node, "progress").unwrap_or("0"))));
            void_tag("input", &common)
        }
        "RatingBar" => {
            common.classes.push("baram-rating".into());
            let stars = number(attr(node, "numStars"), 5.0).max(1.0) as usize;
            let rating = number(attr(node, "rating"), 0.0).round().max(0.0) as usize;
            common.attrs.push(format!("data-stars=\"{stars}\""));
            common.attrs.push(format!("data-rating=\"{}\"", number(attr(node, "rating"), 0.0)));
            let mut content = String::new();
            for i in 1..=stars {
                content.push_str(if i <= rating {
                    "<span class=\"star on\">★</span>"
                } else {
                    "<span class=\"star\">★</span>"
                });
            }
            wrap("div", &common, &content)
        }
        "ProgressBar" => {
            let horizontal = attr(node, "style")
                .or_else(|| node.attribute("style"))
                .map(|x| x.contains("progressBarStyleHorizontal"))
                .unwrap_or(false)
                || attr(node, "indeterminate") == Some("false");
            if horizontal {
                common.classes.push("baram-progress-horizontal".into());
                common.attrs.push(format!("max=\"{}\"", esc_attr(attr(node, "max").unwrap_or("100"))));
                common.attrs.push(format!("value=\"{}\"", esc_attr(attr(node, "progress").unwrap_or("0"))));
                wrap("progress", &common, "")
            } else {
                common.classes.push("baram-progress-spinner".into());
                wrap("div", &common, "")
            }
        }
        "Chronometer" => {
            common.classes.push("baram-text".into());
            common.classes.push("baram-chronometer".into());
            apply_text(node, &mut common);
            wrap("div", &common, "00:00")
        }
        "ImageView" => {
            common.classes.push("baram-image-placeholder".into());
            wrap(
                "div",
                &common,
                &esc_text(attr(node, "contentDescription").unwrap_or("Image")),
            )
        }
        "DatePicker" | "CalendarView" => {
            common.classes.push("baram-date".into());
            common.attrs.push("type=\"date\"".into());
            void_tag("input", &common)
        }
        "TimePicker" => {
            common.classes.push("baram-time".into());
            common.attrs.push("type=\"time\"".into());
            void_tag("input", &common)
        }
        "NumberPicker" => {
            common.classes.push("baram-number-picker".into());
            let min = esc_attr(attr(node, "minValue").unwrap_or("0"));
            let max = esc_attr(attr(node, "maxValue").unwrap_or("100"));
            let value = esc_attr(attr(node, "value").unwrap_or(attr(node, "minValue").unwrap_or("0")));
            wrap(
                "div",
                &common,
                &format!(
                    "<button type=\"button\" data-number-action=\"minus\">−</button><input type=\"number\" min=\"{min}\" max=\"{max}\" value=\"{value}\"><button type=\"button\" data-number-action=\"plus\">+</button>"
                ),
            )
        }
        "ListView" => {
            common.classes.push("baram-list".into());
            let content = (1..=5)
                .map(|i| format!("<div class=\"baram-list-row\">Item {i}</div>"))
                .collect::<String>();
            wrap("div", &common, &content)
        }
        "GridView" => {
            common.classes.push("baram-grid-list".into());
            let cols = number(attr(node, "numColumns"), 2.0).max(1.0) as i64;
            common.styles.push(format!("grid-template-columns:repeat({cols},1fr)"));
            let content = (1..=6)
                .map(|i| format!("<div class=\"baram-grid-cell\">Item {i}</div>"))
                .collect::<String>();
            wrap("div", &common, &content)
        }
        "ExpandableListView" => {
            common.classes.push("baram-expandable".into());
            wrap(
                "div",
                &common,
                "<div class=\"baram-list-row baram-expand-head\">Group 1</div><div class=\"baram-list-row\">Child 1</div><div class=\"baram-list-row\">Child 2</div><div class=\"baram-list-row baram-expand-head\">Group 2</div><div class=\"baram-list-row\">Child 1</div>",
            )
        }
        "SearchView" => {
            common.classes.push("baram-search".into());
            common.attrs.push("type=\"search\"".into());
            common.attrs.push(format!(
                "placeholder=\"{}\"",
                esc_attr(attr(node, "queryHint").unwrap_or("Search"))
            ));
            void_tag("input", &common)
        }
        "WebView" => {
            common.classes.push("baram-web".into());
            wrap("div", &common, "WebView")
        }
        "VideoView" => {
            common.classes.push("baram-video".into());
            wrap("div", &common, "VideoView")
        }
        "ViewFlipper" | "ViewAnimator" | "ViewSwitcher" | "TextSwitcher" => {
            common.classes.push("baram-viewflipper".into());
            common.attrs.push(format!(
                "data-auto-start=\"{}\"",
                bool_attr(node, "autoStart", false)
            ));
            common.attrs.push(format!(
                "data-flip-interval=\"{}\"",
                esc_attr(attr(node, "flipInterval").unwrap_or("3000"))
            ));
            wrap("div", &common, &render_children(node, tag, "")?)
        }
        "Space" => {
            common.classes.push("baram-space".into());
            wrap("div", &common, "")
        }
        "View" => wrap("div", &common, ""),
        _ => {
            // Unknown tags remain visible as generic containers so custom layouts do not disappear.
            common.classes.push("baram-unknown".into());
            common.attrs.push(format!("data-unknown-tag=\"{}\"", esc_attr(tag)));
            let children = render_children(node, tag, "")?;
            wrap("div", &common, &children)
        }
    };

    Ok(html)
}

fn render_children(node: Node<'_, '_>, parent_tag: &str, parent_orientation: &str) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();
    for child in node.children().filter(|n| n.is_element()) {
        out.push_str(&render_node(child, parent_tag, parent_orientation)?);
    }
    Ok(out)
}

fn render_table(node: Node<'_, '_>, parent_tag: &str, parent_orientation: &str) -> Result<String, Box<dyn Error>> {
    let mut common = common_parts(node, parent_tag, parent_orientation);
    common.classes.push("baram-table".into());
    if attr(node, "stretchColumns") == Some("*") {
        common.classes.push("stretch-all".into());
    }

    let mut body = String::from("<tbody>");
    for row in node.children().filter(|n| n.is_element() && n.tag_name().name() == "TableRow") {
        body.push_str("<tr class=\"table-row\"");
        if let Some(h) = attr(row, "layout_height") {
            if h != "wrap_content" {
                body.push_str(&format!(" style=\"height:{}\"", esc_attr(&dim(h))));
            }
        }
        body.push('>');
        for child in row.children().filter(|n| n.is_element()) {
            body.push_str("<td>");
            body.push_str(&render_node(child, "TableRow", "")?);
            body.push_str("</td>");
        }
        body.push_str("</tr>");
    }
    body.push_str("</tbody>");
    Ok(wrap("table", &common, &body))
}

#[derive(Default)]
struct Parts {
    classes: Vec<String>,
    styles: Vec<String>,
    attrs: Vec<String>,
}

fn common_parts(node: Node<'_, '_>, parent_tag: &str, parent_orientation: &str) -> Parts {
    let mut out = Parts::default();
    out.classes.push("baram-view".into());
    out.attrs.push(format!(
        "data-baram-tag=\"{}\"",
        esc_attr(node.tag_name().name())
    ));

    if let Some(id) = attr(node, "id") {
        let name = id_name(id);
        if !name.is_empty() {
            out.attrs.push(format!("data-baram-id=\"{}\"", esc_attr(&name)));
            out.attrs.push(format!("id=\"baram-id-{}\"", esc_attr(&name)));
        }
    }

    if let Some(style_ref) = node.attribute("style").or_else(|| attr(node, "style")) {
        let style_name = style_ref
            .trim_start_matches("@style/")
            .trim_start_matches("?baram:attr/");
        let safe: String = style_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' })
            .collect();
        out.classes.push(format!("style-{safe}"));
    }

    let w = attr(node, "layout_width");
    let h = attr(node, "layout_height");
    let weight = number(attr(node, "layout_weight"), 0.0);

    match w {
        Some("match_parent") | Some("fill_parent") => out.styles.push("width:100%".into()),
        Some("wrap_content") => out.styles.push("width:fit-content".into()),
        Some("0dp") => {}
        Some(v) => out.styles.push(format!("width:{}", dim(v))),
        None => {}
    }
    match h {
        Some("match_parent") | Some("fill_parent") => out.styles.push("height:100%".into()),
        Some("wrap_content") => out.styles.push("height:fit-content".into()),
        Some("0dp") => {}
        Some(v) => out.styles.push(format!("height:{}", dim(v))),
        None => {}
    }

    if parent_tag == "LinearLayout" && weight > 0.0 {
        if parent_orientation == "horizontal" {
            out.styles.push(format!("flex-grow:{weight}"));
            out.styles.push("flex-shrink:1".into());
            out.styles.push(format!("flex-basis:{}", if w == Some("0dp") { "0px" } else { "auto" }));
            if w == Some("0dp") {
                out.styles.push("width:auto".into());
                out.styles.push("min-width:0".into());
            }
        } else {
            out.styles.push(format!("flex-grow:{weight}"));
            out.styles.push("flex-shrink:1".into());
            out.styles.push(format!("flex-basis:{}", if h == Some("0dp") { "0px" } else { "auto" }));
            if h == Some("0dp") {
                out.styles.push("height:auto".into());
                out.styles.push("min-height:0".into());
            }
        }
    } else {
        if w == Some("0dp") {
            out.styles.push("width:0px".into());
        }
        if h == Some("0dp") {
            out.styles.push("height:0px".into());
        }
    }

    apply_sides(node, "padding", "padding", &mut out.styles);
    apply_sides(node, "layout_margin", "margin", &mut out.styles);

    if let Some(bg) = attr(node, "background") {
        if is_css_color(bg) {
            out.styles.push(format!("background:{}", bg));
        }
    }

    if let Some(alpha) = attr(node, "alpha") {
        out.styles.push(format!("opacity:{}", esc_attr(alpha)));
    }

    match attr(node, "visibility") {
        Some("gone") => out.styles.push("display:none".into()),
        Some("invisible") => out.styles.push("visibility:hidden".into()),
        _ => {}
    }

    if attr(node, "enabled") == Some("false") {
        out.styles.push("opacity:.45".into());
        out.styles.push("pointer-events:none".into());
        out.attrs.push("aria-disabled=\"true\"".into());
    }

    for (attr_name, css_name) in [
        ("minWidth", "min-width"),
        ("minHeight", "min-height"),
        ("maxWidth", "max-width"),
        ("maxHeight", "max-height"),
    ] {
        if let Some(v) = attr(node, attr_name) {
            out.styles.push(format!("{css_name}:{}", dim(v)));
        }
    }

    apply_gravity(attr(node, "gravity"), &mut out.styles);

    if parent_tag == "LinearLayout" {
        if let Some(g) = attr(node, "layout_gravity") {
            let terms: Vec<&str> = g.split('|').collect();
            if parent_orientation == "vertical" {
                if terms.contains(&"center") || terms.contains(&"center_horizontal") {
                    out.styles.push("align-self:center".into());
                } else if terms.contains(&"right") || terms.contains(&"end") {
                    out.styles.push("align-self:flex-end".into());
                } else if terms.contains(&"left") || terms.contains(&"start") {
                    out.styles.push("align-self:flex-start".into());
                }
            } else if terms.contains(&"center") || terms.contains(&"center_vertical") {
                out.styles.push("align-self:center".into());
            } else if terms.contains(&"bottom") {
                out.styles.push("align-self:flex-end".into());
            } else if terms.contains(&"top") {
                out.styles.push("align-self:flex-start".into());
            }
        }
    }

    if parent_tag == "AbsoluteLayout" {
        out.styles.push("position:absolute".into());
        out.styles.push(format!("left:{}", dim(attr(node, "layout_x").unwrap_or("0dp"))));
        out.styles.push(format!("top:{}", dim(attr(node, "layout_y").unwrap_or("0dp"))));
    }

    if parent_tag == "GridLayout" {
        if let Some(v) = attr(node, "layout_column") {
            out.styles.push(format!("grid-column-start:{}", number(Some(v), 0.0) as i64 + 1));
        }
        if let Some(v) = attr(node, "layout_row") {
            out.styles.push(format!("grid-row-start:{}", number(Some(v), 0.0) as i64 + 1));
        }
        if let Some(v) = attr(node, "layout_columnSpan") {
            out.styles.push(format!("grid-column-end:span {}", (number(Some(v), 1.0) as i64).max(1)));
        }
        if let Some(v) = attr(node, "layout_rowSpan") {
            out.styles.push(format!("grid-row-end:span {}", (number(Some(v), 1.0) as i64).max(1)));
        }
    }

    // Preserve layout metadata used by the tiny post-render runtime.
    const META: &[&str] = &[
        "layout_gravity",
        "layout_alignParentRight",
        "layout_alignParentEnd",
        "layout_alignParentBottom",
        "layout_centerHorizontal",
        "layout_centerVertical",
        "layout_centerInParent",
        "layout_below",
        "layout_above",
        "layout_toRightOf",
        "layout_toEndOf",
        "layout_toLeftOf",
        "layout_toStartOf",
        "layout_alignLeft",
        "layout_alignStart",
        "layout_alignRight",
        "layout_alignEnd",
        "layout_alignTop",
        "layout_alignBottom",
        "layout_margin",
        "layout_marginHorizontal",
        "layout_marginVertical",
        "layout_marginTop",
        "layout_marginRight",
        "layout_marginEnd",
        "layout_marginBottom",
        "layout_marginLeft",
        "layout_marginStart",
    ];
    for key in META {
        if let Some(value) = attr(node, key) {
            out.attrs.push(format!(
                "data-w3u-{}=\"{}\"",
                key.to_ascii_lowercase(),
                esc_attr(value)
            ));
        }
    }

    out
}

fn apply_text(node: Node<'_, '_>, parts: &mut Parts) {
    if let Some(v) = attr(node, "textSize") {
        parts.styles.push(format!("font-size:{}", dim(v)));
    }
    if let Some(v) = attr(node, "textColor") {
        if is_css_color(v) {
            parts.styles.push(format!("color:{v}"));
        }
    }
    if let Some(v) = attr(node, "textStyle") {
        if v.contains("bold") {
            parts.styles.push("font-weight:700".into());
        }
        if v.contains("italic") {
            parts.styles.push("font-style:italic".into());
        }
    }
    if let Some(v) = attr(node, "maxLines") {
        let lines = number(Some(v), 1.0).max(1.0) as i64;
        parts.styles.push("overflow:hidden".into());
        parts.styles.push("display:-webkit-box".into());
        parts.styles.push("-webkit-box-orient:vertical".into());
        parts.styles.push(format!("-webkit-line-clamp:{lines}"));
    }
}

fn apply_gravity(value: Option<&str>, styles: &mut Vec<String>) {
    let Some(v) = value else { return; };
    let terms: Vec<&str> = v.split('|').collect();
    if terms.contains(&"center") || terms.contains(&"center_horizontal") {
        styles.push("text-align:center".into());
    } else if terms.contains(&"right") || terms.contains(&"end") {
        styles.push("text-align:right".into());
    }
    if terms.contains(&"center") || terms.contains(&"center_vertical") {
        styles.push("display:flex".into());
        styles.push("align-items:center".into());
        if terms.contains(&"center") {
            styles.push("justify-content:center".into());
        }
    }
    if terms.contains(&"bottom") {
        styles.push("display:flex".into());
        styles.push("align-items:flex-end".into());
    }
}

fn apply_sides(node: Node<'_, '_>, base: &str, css_name: &str, styles: &mut Vec<String>) {
    let all = attr(node, base);
    let horizontal_key = format!("{base}Horizontal");
    let vertical_key = format!("{base}Vertical");
    let h = attr(node, &horizontal_key);
    let v = attr(node, &vertical_key);

    let side = |name: &str, alt: Option<&str>| -> String {
        let direct = attr(node, &format!("{base}{name}"));
        let alternate = alt.and_then(|a| attr(node, &format!("{base}{a}")));
        dim(direct.or(alternate).or(if matches!(name, "Top" | "Bottom") { v } else { h }).or(all).unwrap_or("0dp"))
    };

    let top = side("Top", None);
    let right = side("Right", Some("End"));
    let bottom = side("Bottom", None);
    let left = side("Left", Some("Start"));

    if [top.as_str(), right.as_str(), bottom.as_str(), left.as_str()]
        .iter()
        .any(|x| *x != "0px")
    {
        styles.push(format!("{css_name}:{top} {right} {bottom} {left}"));
    }
}

fn attr<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.name() == name)
        .map(|a| a.value())
}

fn bool_attr(node: Node<'_, '_>, name: &str, default: bool) -> bool {
    attr(node, name)
        .map(|x| x.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

fn number(value: Option<&str>, default: f64) -> f64 {
    value
        .and_then(|v| {
            let numeric: String = v
                .chars()
                .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
                .collect();
            numeric.parse::<f64>().ok()
        })
        .unwrap_or(default)
}

fn dim(value: &str) -> String {
    match value {
        "match_parent" | "fill_parent" => "100%".into(),
        "wrap_content" => "auto".into(),
        _ => {
            let trimmed = value.trim();
            let suffixes = ["dp", "dip", "sp", "px"];
            for suffix in suffixes {
                if let Some(n) = trimmed.strip_suffix(suffix) {
                    if let Ok(v) = n.parse::<f64>() {
                        return format_px(v);
                    }
                }
            }
            if let Ok(v) = trimmed.parse::<f64>() {
                return format_px(v);
            }
            trimmed.to_owned()
        }
    }
}

fn format_px(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}px", v as i64)
    } else {
        format!("{v}px")
    }
}

fn is_css_color(value: &str) -> bool {
    let v = value.trim();
    v.starts_with('#')
        || v.starts_with("rgb(")
        || v.starts_with("rgba(")
        || v.chars().all(|c| c.is_ascii_alphabetic())
}

fn id_name(value: &str) -> String {
    value
        .trim_start_matches("@+id/")
        .trim_start_matches("@id/")
        .trim_start_matches("@baram:id/")
        .to_owned()
}

fn wrap(tag: &str, parts: &Parts, content: &str) -> String {
    format!(
        "<{tag}{}>{content}</{tag}>",
        attrs_to_string(parts)
    )
}

fn void_tag(tag: &str, parts: &Parts) -> String {
    format!("<{tag}{}>", attrs_to_string(parts))
}

fn attrs_to_string(parts: &Parts) -> String {
    let mut attrs = parts.attrs.clone();
    if !parts.classes.is_empty() {
        attrs.push(format!("class=\"{}\"", esc_attr(&parts.classes.join(" "))));
    }
    if !parts.styles.is_empty() {
        attrs.push(format!("style=\"{}\"", esc_attr(&parts.styles.join(";"))));
    }
    if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    }
}

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn esc_attr(s: &str) -> String {
    esc_text(s).replace('"', "&quot;")
}

fn build_document(body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<title>W3U Output</title>
<style>{CSS}</style>
</head>
<body>
<div id="app">{body}</div>
<script>{RUNTIME_JS}</script>
</body>
</html>
"#
    )
}

pub const CSS: &str = r#"
*{box-sizing:border-box}
html,body,#app{width:100%;height:100%;margin:0;padding:0}
html,body{overflow:hidden}
body{font-family:Roboto,"Helvetica Neue",Arial,sans-serif;font-size:14px;background:#fff;color:#222;-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}
#app{overflow:hidden}
.baram-view{box-sizing:border-box;min-width:0;min-height:0;gap:8px}
.baram-linear{display:flex;min-width:0;min-height:0;align-items:stretch}
.baram-scroll{min-width:0;min-height:0;overscroll-behavior:contain;-webkit-overflow-scrolling:touch}
.baram-relative,.baram-frame,.baram-absolute{position:relative;min-width:0;min-height:0}
.baram-frame>.baram-view{position:absolute}
.baram-grid{display:grid;min-width:0;min-height:0}
.baram-table{border-collapse:separate;border-spacing:6px;width:auto;max-width:100%}
.baram-table.stretch-all{width:100%;table-layout:fixed}
.baram-table td{vertical-align:middle;padding:0;min-width:0}
.baram-table.stretch-all td>.baram-view{width:100%!important;max-width:100%}
.baram-space{display:block}
.style-SectionTitle{width:100%!important;margin:22px 0 8px!important;padding:0!important;font-size:24px!important;line-height:30px!important;font-weight:700!important}
.style-SectionDescription{width:100%!important;margin:0 0 14px!important;padding:0!important;font-size:13px!important;line-height:18px!important;color:#666}
.style-ComponentLabel{width:100%!important;margin:14px 0 6px!important;padding:0!important;font-size:15px!important;line-height:20px!important;color:#333}
.style-ComponentBox{margin-bottom:3px}
.baram-text{display:block;white-space:pre-wrap;line-height:1.25}
.baram-button,.baram-image-button,.baram-toggle{-webkit-appearance:none;appearance:none;position:relative;min-width:64px;min-height:48px;margin:0;padding:0 16px;border:1px solid #aaa;border-radius:2px;outline:none;background:linear-gradient(#fafafa,#e5e5e5);color:#222;font:400 18px/46px Roboto,"Helvetica Neue",Arial,sans-serif;text-align:center;cursor:pointer;user-select:none}
.baram-primary-button{-webkit-appearance:none;appearance:none;position:relative;min-width:64px;min-height:48px;margin:0;padding:0 16px;border:1px solid #007dff;border-radius:2px;outline:none;background:#007dff;color:#fff;font:400 18px/46px Roboto,"Helvetica Neue",Arial,sans-serif;text-align:center;cursor:pointer;user-select:none}
.baram-primary-button:hover{background:#0070e8;border-color:#0070e8}.baram-primary-button:active{background:#0060c4;border-color:#0060c4}
.baram-button:active,.baram-image-button:active,.baram-toggle:active{background:#d6d6d6}
.baram-edit{-webkit-appearance:none;appearance:none;display:block;min-width:80px;min-height:38px;margin:0;padding:7px 4px 5px;border:0;border-bottom:2px solid #33b5e5;border-radius:0;outline:none;background:transparent;color:#222;font:400 16px/22px Roboto,"Helvetica Neue",Arial,sans-serif}
textarea.baram-edit{min-height:58px;resize:vertical;border:1px solid #aaa;border-bottom:2px solid #33b5e5;padding:7px}
.baram-checkbox,.baram-radio{display:inline-flex;align-items:center;gap:10px;min-height:42px;padding:2px 0;cursor:pointer;user-select:none;outline:none}
.baram-checkbox .mark{width:22px;height:22px;position:relative;flex:0 0 22px;border:2px solid #777;border-radius:1px}
.baram-checkbox[data-checked="true"] .mark{border-color:#33b5e5}
.baram-checkbox[data-checked="true"] .mark:after{content:"";position:absolute;left:5px;top:1px;width:7px;height:13px;border:solid #33b5e5;border-width:0 3px 3px 0;transform:rotate(45deg)}
.baram-radio .mark{width:22px;height:22px;position:relative;flex:0 0 22px;border:2px solid #777;border-radius:50%}
.baram-radio[data-checked="true"] .mark{border-color:#33b5e5}
.baram-radio[data-checked="true"] .mark:after{content:"";position:absolute;inset:4px;border-radius:50%;background:#33b5e5}
.baram-switch{display:inline-flex;align-items:center;gap:11px;min-height:44px;padding:2px 0;cursor:pointer;user-select:none;outline:none}
.baram-switch .track{width:52px;height:22px;position:relative;flex:0 0 52px;border:1px solid #888;background:#aaa}
.baram-switch .thumb{position:absolute;left:-1px;top:-4px;width:28px;height:28px;border:1px solid #777;background:#eee;transition:left .1s linear}
.baram-switch[data-checked="true"] .track{background:#7ed9ef;border-color:#33b5e5}
.baram-switch[data-checked="true"] .thumb{left:25px;background:#33b5e5;border-color:#1688a4}
.baram-spinner{-webkit-appearance:auto;appearance:auto;min-height:38px;padding:6px 30px 5px 7px;border:0;border-bottom:2px solid #33b5e5;border-radius:0;outline:none;background:#fff;color:#222;font:400 15px Roboto,"Helvetica Neue",Arial,sans-serif}
.baram-seek{width:100%;height:30px;margin:0;accent-color:#33b5e5}
.baram-rating{display:inline-flex;gap:1px;align-items:center;min-height:34px;font-size:25px;line-height:1;user-select:none;cursor:pointer;width:fit-content}
.baram-rating .star{color:#aaa}.baram-rating .star.on{color:#f3b400}
.baram-progress-horizontal{width:100%;height:6px;border:0;accent-color:#33b5e5}
.baram-progress-spinner{width:28px;height:28px;border-radius:50%;border:4px solid #ccc;border-top-color:#33b5e5;animation:w3u-spin .8s linear infinite}
@keyframes w3u-spin{to{transform:rotate(360deg)}}
.baram-image-placeholder{display:flex;align-items:center;justify-content:center;min-width:56px;min-height:48px;border:1px solid #aaa;background:#eee;font-size:12px}
.camera-icon{position:relative;display:inline-block;width:24px;height:17px;border:2px solid #555;border-radius:2px}.camera-icon:before{content:"";position:absolute;left:6px;top:-6px;width:8px;height:5px;border:2px solid #555;border-bottom:0}.camera-icon:after{content:"";position:absolute;left:7px;top:3px;width:7px;height:7px;border:2px solid #555;border-radius:50%}
.baram-list,.baram-expandable{overflow:auto;border-top:1px solid #ccc;border-bottom:1px solid #ccc}.baram-list-row{min-height:44px;display:flex;align-items:center;padding:0 12px;border-bottom:1px solid #ddd}.baram-list-row:last-child{border-bottom:0}.baram-expand-head{font-weight:500;background:#eee}
.baram-grid-list{display:grid;gap:1px;overflow:auto}.baram-grid-cell{min-height:48px;display:flex;align-items:center;justify-content:center;background:#eee}
.baram-search{width:100%;min-height:38px;padding:7px 8px 5px;border:0;border-bottom:2px solid #33b5e5;outline:none;font:400 15px Roboto,"Helvetica Neue",Arial,sans-serif}
.baram-number-picker{display:inline-flex;align-items:center;width:fit-content;border:1px solid #aaa}.baram-number-picker button{width:34px;height:34px;border:0;font-size:18px;cursor:pointer}.baram-number-picker input{width:60px;height:34px;border:0;border-left:1px solid #aaa;border-right:1px solid #aaa;text-align:center}
.baram-date,.baram-time{min-height:38px;padding:5px;border:1px solid #aaa;font:14px Roboto,Arial,sans-serif}
.baram-web,.baram-video{display:flex;align-items:center;justify-content:center;border:1px solid #aaa;background:#eee}.baram-video:before{content:"▶";font-size:22px;margin-right:8px}
.baram-viewflipper{position:relative;overflow:hidden}.baram-viewflipper>.baram-view{position:absolute!important;left:0!important;top:0!important;right:0!important;bottom:0!important;opacity:0;pointer-events:none}.baram-viewflipper>.baram-view.active{opacity:1;pointer-events:auto}
.baram-unknown:empty:before{content:attr(data-unknown-tag);font:12px monospace;color:#999}
.baram-scroll::-webkit-scrollbar{width:6px;height:6px}.baram-scroll::-webkit-scrollbar-track{background:transparent}.baram-scroll::-webkit-scrollbar-thumb{background:#999}
"#;

pub const RUNTIME_JS: &str = r#"
"use strict";
const q=(s,r=document)=>Array.from(r.querySelectorAll(s));
const truth=v=>String(v).toLowerCase()==="true";
const num=(v,d=0)=>{const n=parseFloat(v);return Number.isFinite(n)?n:d};
const refName=v=>(v||"").replace(/^@\+?id\//,"").replace(/^@id\//,"").replace(/^@baram:id\//,"");
const meta=(el,k)=>el.getAttribute("data-w3u-"+k.toLowerCase());
function setChecked(el,v){el.dataset.checked=v?"true":"false";el.setAttribute("aria-checked",v?"true":"false")}
function activate(el,fn){el.addEventListener("click",e=>{e.preventDefault();e.stopPropagation();fn(e)});el.addEventListener("keydown",e=>{if(e.key===" "||e.key==="Enter"){e.preventDefault();e.stopPropagation();fn(e)}})}
q(".baram-checkbox").forEach(el=>{setChecked(el,truth(el.dataset.checked));activate(el,()=>setChecked(el,!truth(el.dataset.checked)))});
q(".baram-radio").forEach(el=>{setChecked(el,truth(el.dataset.checked));activate(el,()=>{const group=el.closest(".baram-radio-group");if(group)q(":scope > .baram-radio",group).forEach(x=>setChecked(x,x===el));else setChecked(el,true)})});
q(".baram-radio-group").forEach(group=>{const radios=q(":scope > .baram-radio",group);const selected=radios.find(x=>truth(x.dataset.checked));if(selected)radios.forEach(x=>setChecked(x,x===selected))});
q(".baram-switch").forEach(el=>{setChecked(el,truth(el.dataset.checked));activate(el,()=>setChecked(el,!truth(el.dataset.checked)))});
q(".baram-toggle").forEach(el=>{const sync=()=>{const checked=truth(el.dataset.checked);el.setAttribute("aria-pressed",checked?"true":"false");el.textContent=checked?el.dataset.textOn:el.dataset.textOff};sync();el.addEventListener("click",()=>{el.dataset.checked=truth(el.dataset.checked)?"false":"true";sync()})});
q(".baram-rating").forEach(el=>el.addEventListener("click",e=>{const count=Math.max(1,num(el.dataset.stars,5));const r=el.getBoundingClientRect();const next=Math.max(0,Math.min(count,Math.ceil((e.clientX-r.left)/Math.max(1,r.width)*count)));q(".star",el).forEach((s,i)=>s.classList.toggle("on",i<next));el.dataset.rating=String(next)}));
q(".baram-number-picker").forEach(el=>{const input=el.querySelector("input");el.addEventListener("click",e=>{const action=e.target?.getAttribute?.("data-number-action");if(action==="minus")input.stepDown();if(action==="plus")input.stepUp()})});
q(".baram-chronometer").forEach(el=>{const start=performance.now();const tick=()=>{if(!el.isConnected)return;const sec=Math.floor((performance.now()-start)/1000);el.textContent=String(Math.floor(sec/60)).padStart(2,"0")+":"+String(sec%60).padStart(2,"0");setTimeout(tick,250)};tick()});
q(".baram-viewflipper").forEach(el=>{const kids=Array.from(el.children);if(!kids.length)return;let i=0;kids[0].classList.add("active");if(truth(el.dataset.autoStart)){const delay=Math.max(250,num(el.dataset.flipInterval,3000));setInterval(()=>{if(!el.isConnected)return;kids[i].classList.remove("active");i=(i+1)%kids.length;kids[i].classList.add("active")},delay)}});
function margins(el){const all=meta(el,"layout_margin"),h=meta(el,"layout_marginhorizontal"),v=meta(el,"layout_marginvertical");return{top:num(meta(el,"layout_margintop")||v||all),right:num(meta(el,"layout_marginright")||meta(el,"layout_marginend")||h||all),bottom:num(meta(el,"layout_marginbottom")||v||all),left:num(meta(el,"layout_marginleft")||meta(el,"layout_marginstart")||h||all)}}
function layoutRelative(el){const kids=Array.from(el.children);const ids={};kids.forEach(h=>{if(h.dataset.baramId)ids[h.dataset.baramId]=h});const pr=el.getBoundingClientRect();kids.forEach(ch=>{ch.style.position="absolute";const r=ch.getBoundingClientRect();const m=margins(ch);let x=m.left,y=m.top;if(truth(meta(ch,"layout_alignparentright"))||truth(meta(ch,"layout_alignparentend")))x=pr.width-r.width-m.right;if(truth(meta(ch,"layout_centerhorizontal"))||truth(meta(ch,"layout_centerinparent")))x=(pr.width-r.width)/2;if(truth(meta(ch,"layout_alignparentbottom")))y=pr.height-r.height-m.bottom;if(truth(meta(ch,"layout_centervertical"))||truth(meta(ch,"layout_centerinparent")))y=(pr.height-r.height)/2;const below=ids[refName(meta(ch,"layout_below"))],above=ids[refName(meta(ch,"layout_above"))],right=ids[refName(meta(ch,"layout_torightof")||meta(ch,"layout_toendof"))],left=ids[refName(meta(ch,"layout_toleftof")||meta(ch,"layout_tostartof"))];if(below){const q=below.getBoundingClientRect();y=q.bottom-pr.top+m.top}if(above){const q=above.getBoundingClientRect();y=q.top-pr.top-r.height-m.bottom}if(right){const q=right.getBoundingClientRect();x=q.right-pr.left+m.left}if(left){const q=left.getBoundingClientRect();x=q.left-pr.left-r.width-m.right}const al=ids[refName(meta(ch,"layout_alignleft")||meta(ch,"layout_alignstart"))],ar=ids[refName(meta(ch,"layout_alignright")||meta(ch,"layout_alignend"))],at=ids[refName(meta(ch,"layout_aligntop"))],ab=ids[refName(meta(ch,"layout_alignbottom"))];if(al)x=al.getBoundingClientRect().left-pr.left+m.left;if(ar)x=ar.getBoundingClientRect().right-pr.left-r.width-m.right;if(at)y=at.getBoundingClientRect().top-pr.top+m.top;if(ab)y=ab.getBoundingClientRect().bottom-pr.top-r.height-m.bottom;ch.style.left=Math.max(0,x)+"px";ch.style.top=Math.max(0,y)+"px"})}
function layoutFrame(el){const pr=el.getBoundingClientRect();Array.from(el.children).forEach(ch=>{const r=ch.getBoundingClientRect();const g=(meta(ch,"layout_gravity")||"top|left").split("|");let x=0,y=0;if(g.includes("center")||g.includes("center_horizontal"))x=(pr.width-r.width)/2;else if(g.includes("right")||g.includes("end"))x=pr.width-r.width;if(g.includes("center")||g.includes("center_vertical"))y=(pr.height-r.height)/2;else if(g.includes("bottom"))y=pr.height-r.height;ch.style.left=Math.max(0,x)+"px";ch.style.top=Math.max(0,y)+"px"})}
function relayout(){q(".baram-relative").forEach(layoutRelative);q(".baram-frame").forEach(layoutFrame)}
requestAnimationFrame(relayout);window.addEventListener("resize",()=>requestAnimationFrame(relayout));
"#;
