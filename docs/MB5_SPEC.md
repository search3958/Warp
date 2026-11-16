# MB5 language and runtime specification (next foundation)

Version: MB5-NX 0.1 (foundational implementation)

This document describes the behavior implemented by the current Rust runner and the embedded script VM. It is intentionally a bounded application format, not a claim of full SwiftUI or Tcl compatibility.

## 1. Design principles

1. **Declarative UI first.** UI files describe structure and presentation; scripts describe behavior.
2. **Opinionated defaults.** The standard light theme, spacing, corner treatment, typography, and component appearance are the preferred design. Per-component overrides are intentionally limited.
3. **Small runtime.** The runner keeps fixed-size project and event structures, renders to a software framebuffer, and does not require a GPU UI toolkit.
4. **Host capabilities are explicit.** Network, UI mutation, logging, and timers are exposed as named host commands rather than arbitrary native access.
5. **Incremental evolution.** Existing `@mainBoard`, `board`, `card`, and `@mainScript` files remain readable. A separate script file is preferred for new projects.

## 2. Project layout

```text
components/
  config.ini
  app/main.mb5
  models/main.yaml
  scripts/main.mb5
  resources/...
```

`config.ini` currently supports:

```ini
version=5
name=My MosaicBoard app
icon=icon.svg
primaryColor=#006AFF
```

`primaryColor` accepts a six-digit RGB hex color. The default is `#006AFF`. The theme is deliberately opinionated; primary color is the main project-wide customization point, not a general design-token editor.

The runner loads the UI file, optional YAML model, and then optional `scripts/main.mb5`. When `scripts/main.mb5` exists it takes precedence over the legacy inline `@mainScript` section. The VM evaluates the complete script file from top to bottom: top-level initialization commands run immediately, procedure definitions are registered, and `main()` is invoked automatically when present. A separate script can therefore start with `set`, `let`, `dict`, or other initialization commands; it does not have to begin with `proc`. Legacy inline scripts remain on the compatibility parser unless they explicitly start with the newer `proc` / `fn` syntax.

## 3. UI language

### 3.1 Board and component declarations

```text
@mainBoard
board home {
    topbar {
        horizontal {
            icon(home)
            text(config:text:"My app")
        }
    }

    vertical {
        card {
            text(config:title:"Welcome")
            text(config:text:"Hello")
            button(config:text:"Continue", event: continueApp())
        }
    }
}
```

A board is a named screen. Containers use braces and form a parent-child tree. Simple components use `kind(name)` or `kind(...)`. Component properties are placed on following lines and apply to the immediately preceding component.

### 3.2 Supported components

| Component | Purpose | Current behavior |
|---|---|---|
| `icon(name)` | Standard symbol | Built-in vector-like symbols; unknown names use a simple fallback glyph |
| `input(name)` | Text input | Focus, text insertion, backspace, placeholder, value, event |
| `checkbox(name)` | Boolean option | Toggle on click; state readable by script |
| `switch(name)` | Boolean option | Switch presentation of the same boolean state |
| `slider(name)` | Numeric input | Min/max, click and drag, event on value change |
| `progress(name)` | Progress indicator | Value in the 0–100 range |
| `divider()` | Section separator | Neutral one-pixel divider |
| `scrollbox { ... }` | Bounded scrolling region | Fixed-height clipping and its own scroll offset |
| `topbar { ... }` | Board top bar | Root-level only; automatically laid out and unique per board |
| `navigation(name)` | Navigation choices | Items from a YAML model; selected item event |
| `dropdown(name)` | Menu selection | Expands and selects YAML model items |
| `tab(name)` | Tab selection | Selected underline and model events |
| `spacing(px)` | Empty vertical spacing | Numeric spacing in logical pixels |
| `horizontal { ... }` | Horizontal stack | Equally distributed child columns |
| `vertical { ... }` | Vertical stack | Ordered child layout |
| `grid { ... }` | Grid layout | Configurable column count |
| `snackbar(name)` | Transient message surface | Fixed bottom overlay; does not consume document layout space; control visibility via script |
| `gradient { ... }` | Gradient surface | Two-color linear surface |
| `opacity { ... }` | Alpha group | Applies opacity to child content |

Existing `text`, `list`, `button`, `card`, and `alert` components remain supported.

`topbar` is intentionally special: it must be a direct child of a board, may be declared only once per board, and gets its position from the runtime. Do not position it manually or place it inside another container. The parser records a diagnostic and ignores invalid duplicate/nested declarations.

### 3.3 Common modifiers

The current parser recognizes these modifiers where appropriate:

| Modifier | Meaning |
|---|---|
| `.id("stable-id")` | Script-addressable component ID; unique within a project is recommended |
| `.display(show)` / `.display(hide)` | Initial visibility |
| `.value(value)` | Initial input, toggle, slider, or progress value |
| `.placeholder("text")` | Input placeholder or control label |
| `.min(number)` / `.max(number)` | Slider bounds |
| `.height(number)` | Explicit container/scrollbox height |
| `.width(value)` | Button width hint |
| `.size(value)` | Button size preset |
| `.radius(number)` | Explicit corner radius override |
| `.bgColor(#RRGGBB)` | Surface color |
| `.fgColor(#RRGGBB)` | Foreground color |
| `.opacity(0..100)` | Group/component opacity |
| `.gradient(#RRGGBB,#RRGGBB)` | Gradient start and end colors |
| `.columns(1..8)` | Grid column count |
| `.spacing(number)` | Container spacing |
| `.transition(milliseconds)` | Color/gradient/opacity interpolation duration |
| `.type(primary)` / `.type(text)` | Button visual role |

The visual system favors the built-in style. Use modifiers only when a component needs a meaningful semantic distinction. Rounded surfaces use the shared Figma-like squircle cubic path from the SmoothCornerShape control-point formula, including rounded gradient surfaces. A separate transition component is not defined: `.transition(ms)` is a modifier. Its default duration is 180 ms; `.transition(0)` disables interpolation for that component.

### 3.4 Transition boundary

Transitions are intentionally limited to:

- foreground color;
- background color;
- gradient end color (and therefore its displayed gradient);
- opacity.

Transitions do **not** animate position, dimensions, shape geometry, scrolling, or arbitrary transforms. Those are reserved for a future explicit animation system so that the visual behavior remains predictable and later custom animations can be introduced without conflicting with `transition`.

### 3.5 UI events

A component can declare an event inline:

```text
button(config:text:"Load", event: loadRemote())
input(search)
.id("search-field")
.event: searchChanged()
```

For models, YAML `events` entries are linked to the matching model IDs:

```yaml
sample-navigation:
  model:
    home: "Home"
    settings: "Settings"
  events:
    home: showHome()
    settings: showSettings()
```

The event procedure is called on click, selection, toggle, slider change, or text input according to the component. `ui.get` reads the latest component state.

## 4. Script language

### 4.1 Language lineage

The VM uses a compact, command-oriented, Tcl/Picol-shaped syntax. It is an original MB5 implementation, not a vendored Picol interpreter. It adds selected features useful for UI applications: list/dictionary helpers, JSON field extraction, structured error capture, host UI commands, timed callbacks, and bounded HTTP requests.

### 4.2 Words and command boundaries

- Newline or `;` separates commands.
- Spaces/tabs separate words.
- `{ ... }` creates a literal block and supports nested braces.
- `"..."` creates an interpolated quoted word.
- `$name` and `${name}` substitute variable values.
- `[command ...]` substitutes the result of a nested command.
- `#` begins a comment when it appears at the beginning of a command.

Example:

```text
proc greet {name} {
    puts "Hello, $name"
}
```

### 4.3 Variables and scopes

```text
set title "Welcome"
let count 0
incr count
append title " to MB5"
```

`set` and `let` are equivalent. Assignment in a procedure creates a local variable. Lookup searches the current and parent procedure frames before globals. Names beginning with an uppercase ASCII letter are global. `global name` explicitly declares a global variable inside a procedure.

The current implementation stores values as bounded UTF-8 strings. Numbers are parsed for arithmetic when needed; this is not yet a statically typed or bytecode-compiled language. `string length` counts UTF-8 bytes, and `string lower/upper` perform byte-oriented ASCII case conversion. This preserves Picol's small runtime character while leaving room for a future typed value representation.

### 4.4 Procedures, conditions, and loops

```text
proc add {left right} {
    return [expr $left + $right]
}

proc repeatDemo {} {
    set index 0
    while {$index < 3} {
        puts "item $index"
        incr index
    }
}

proc listDemo {} {
    foreach item [list alpha beta gamma] {
        puts $item
    }
}
```

Supported statements/commands:

- `proc` / `fn`, `return`;
- `if`, `elseif`, `else`;
- `while`, `foreach`, `break`, `continue`;
- `expr`, `incr`, `append`;
- `list`, `llength`, `lindex`, `lappend`;
- `dict create/get/set/exists/keys`;
- `string length/trim/contains/equal/startsWith/lower/upper`;
- `catch`, `error`, `assert`;
- `puts` / `print`.

Arithmetic expressions currently support numeric literals and variables, parentheses, unary `!` and `-`, `*`, `/`, `+`, `-`, comparisons, `&&`, and `||`. String comparison should use `string equal`; values are not implicitly converted to a full dynamic type system.

### 4.5 JSON handling

```text
set title [json.get $response title]
set version [json.get $response {meta.version}]
set firstName [json.get $response {items[0].name}]
set hasVersion [json.has $response {meta.version}]
```

`json.get` and `json.has` support dotted object paths and zero-based array indexes (`items[0].name`). If a path resolves to an object or array, the value is returned as raw JSON text. Unicode `\uXXXX` escapes are preserved literally rather than decoded.

### 4.6 UI bridge

```text
ui.set status-label text "Loaded"
ui.set status-snackbar display show
set currentValue [ui.get search-field value]
```

Supported `ui.set` properties: `text`, `title`, `message`, `description`, `value`, `display`, `checked`, `opacity`, `bgColor`, `fgColor`, `gradientEnd`.

Supported `ui.get` properties: `text`, `title`, `message`, `description`, `value`, `display`, `checked`, `opacity`, `bgColor`, `fgColor`.

The target is matched by `.id(...)`. Unknown IDs/properties return an error instead of being silently ignored. `ui.set` starts the component's `.transition(ms)` when an interpolatable property is changed.

### 4.7 Timers

Prefer non-blocking `after`:

```text
proc main {} {
    after 1000 showStatus
}
proc showStatus {} {
    ui.set status-label text "One second elapsed"
}
```

`after milliseconds procedure ?argument?` schedules a callback on the UI event loop. There are 16 pending callback slots and the delay limit is one hour. Callbacks execute on the main UI thread. `wait milliseconds` is retained for compatibility but blocks the calling thread and is capped at 30 seconds; avoid it in interactive UI code.

### 4.8 HTTP fetch

```text
set response [fetch "https://api.example.com/items/1"]
set title [json.get $response title]

set payload "{\"name\":\"MosaicBoard\"}"
set response [fetch POST "https://httpbin.org/post" $payload "application/json"]
```

Supported methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`.

Full signature:

```text
fetch URL
fetch METHOD URL ?BODY? ?CONTENT_TYPE? ?HEADERS?
```

Headers are newline-separated `Name: value` lines. Only `http://` and `https://` URLs are allowed. The runner invokes the system `curl` executable; it does not embed a TLS stack. Redirects are limited to five and protocols are restricted to HTTP(S). Connect timeout is 5 seconds, total timeout 15 seconds, request body is capped at 4 KiB, response at 16 KiB, and the VM result buffer is bounded. HTTP 4xx/5xx responses are returned normally so scripts can inspect `HTTP_STATUS`; transport/size failures produce an error and populate `HTTP_ERROR`.

**Important current limitation:** `fetch` is synchronous, so the UI event loop can pause for up to the request timeout. Use it for small, user-initiated requests only. A future worker/async host API is needed for background networking. Do not put secrets directly in the UI file; environment-variable access and secure credential storage are not part of this baseline.

## 5. Runtime limits and safety

The implementation is intentionally bounded. Current script limits include 48 procedures, 96 global variables, 32 local variables per frame, 24 arguments, 32 nested calls, and 10,000 loop iterations. Values/commands/response data have fixed maximum sizes. The Rust runner's project and component tables also have fixed capacities.

The fetch bridge validates protocols, methods, request sizes, header names/values, shell-quotes dynamic arguments, and applies network timeouts. It is not a browser sandbox: the script can request any HTTP(S) host reachable from the machine, including loopback or private-network services. Treat projects/scripts as trusted local code, do not execute untrusted projects, and validate remote data before displaying it. Private-network destination filtering and DNS-rebinding defenses are not implemented in this foundation.

## 6. Compatibility and implementation status

This is a runnable foundation and preview runtime, not a complete production compiler. Existing MB5 syntax remains supported where implemented, but unrecognized YAML features, arbitrary YAML documents, Unicode decoding for JSON `\uXXXX` escapes, async networking, a package/module system, static typing, and user-defined animation curves are outside this baseline.

`tools/mb5/check.sh` runs the UI structural lint, C VM unit tests, and script syntax check. It runs `cargo check` when Cargo is available. The C VM tests can be run independently without Rust.
