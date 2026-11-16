# MosaicBoard 5 development guide

## Requirements

- Rust toolchain (`rustup`, `cargo`, `rustc`) for the runner.
- A C compiler (`cc` or `clang`) for the embedded script VM and its tests.
- `curl` at runtime for the `fetch` command.
- macOS for the current interactive AppKit window backend. Other supported targets can use `--ppm` for an off-screen preview.
- Python 3 for the project lint utility. No third-party Python packages are required.

## Build and run

Run from the project root:

```sh
sh ./runner/rust/run.sh
sh ./runner/rust/run.sh 50%
sh ./runner/rust/run.sh ./example/components
sh ./runner/rust/run.sh ./example/components --ppm > preview.ppm
```

The percent profile is a software execution/rendering profile, not a CPU frequency setting. `100%` is normal speed; lower values reduce the frame cap and legacy inline-script budget and stretch legacy `wait`. This is not a CPU-clock limit and does not strictly throttle the new C VM procedures or `fetch`.

## Validation

```sh
sh ./tools/mb5/check.sh
```

The checker verifies the example's MB5 delimiters, topbar placement/uniqueness, and component IDs; compiles and runs VM unit tests; parses the script file without executing its procedures or issuing HTTP requests; and runs `cargo check` when Cargo is installed. The C tests do not prove the Rust host bridge compiles.

## Project architecture

| Path | Responsibility |
|---|---|
| `runner/rust/src/project.rs` | Fixed-capacity project parser and YAML model binding |
| `runner/rust/src/app.rs` | Layout, software drawing orchestration, hit testing, controls, transitions, callbacks |
| `runner/rust/src/canvas.rs` | Pixel framebuffer and vector-like drawing primitives |
| `runner/rust/src/font.rs` | Bundled font rasterization and glyph cache |
| `runner/rust/src/script.rs` | Rust/C FFI host bridge, script VM lifecycle, fetch/UI/timer bindings |
| `runner/rust/src/scriptvm/mb5_vm.c` | Picol-inspired command interpreter |
| `runner/rust/src/sys.rs` | Minimal libc wrappers and bounded curl invocation |
| `example/components/app/main.mb5` | Declarative UI example |
| `example/components/models/main.yaml` | Navigation, dropdown, and tab models |
| `example/components/scripts/main.mb5` | Startup, callbacks, UI binding, JSON/fetch example |
| `tools/mb5/` | Project lint and VM validation tools |
| `tools/vscode/` | Optional VS Code language extension source |

## Adding a component

1. Add the component keyword to `is_component_kind()` and, when it owns children, `is_container_kind()` in `project.rs`.
2. Define parse-time state in `Comp` and `EMPTY_COMP`.
3. Implement its height/layout in `App::component_height()` and `container_content_height()` as appropriate.
4. Implement drawing and hit areas in `App::draw_component()`.
5. Add interaction to the `Action` enum and `App::click()` / pointer methods.
6. Add a sample in `example/components/app/main.mb5` and document it in `docs/MB5_SPEC.md`.
7. Run `tools/mb5/check.sh`; then build with Cargo on macOS and verify mouse, keyboard, scroll, and resize behavior.

## Adding a script command

1. Implement the command in `eval_command()` in `scriptvm/mb5_vm.c`.
2. Keep capacity checks explicit; return `VM_ERROR` with a useful `set_result()` message on invalid arguments.
3. Add the command to `script_vm_test.c` using small deterministic input.
4. If the command calls the host, declare a narrow function in `mb5_vm.h` and implement it in `script.rs`.
5. Document the signature, bounds, and blocking behavior in `MB5_SPEC.md`.

## VS Code language support

The source extension in `tools/vscode` provides `.mb5` language association, basic syntax highlighting, bracket pairs, and snippets. To test it locally, open `tools/vscode` as an Extension Development Host using VS Code's `--extensionDevelopmentPath` option, or package it with `vsce` if installed. `vsce` is optional and is not a runtime dependency.

## Known development caveats

- The Rust target is `#![no_std]` / `#![no_main]`, and its fixed buffers should remain bounded. Avoid silently truncating user data; return a diagnostic instead.
- The script VM is C99 and values are currently strings. Add type features deliberately rather than mixing ad hoc representations.
- `fetch` and legacy `wait` are synchronous. The new `after` timer is non-blocking and should be preferred. Timer deadlines use monotonic wall-clock time and are not changed by the speed profile.
- The script scheduler and active project bridge currently assume a single UI thread and one active `App`/VM instance.
- The static lint is structural, not a full MB5 parser. C VM script checking is authoritative only for the currently implemented script grammar.
