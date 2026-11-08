# Mosaic Board 5 Runner (Rust / no_std)

`components/` を、GUIエンジン・GPU・std を使わずにピクセル単位で描画して開くランナーです。

## 起動

プロジェクトのルート（`runner/` と `example/` がある場所）で実行します。

```sh
sh ./runner/rust/run.sh                         # ./example/components を開く（なければ ./components）
sh ./runner/rust/run.sh path/to/components      # 指定した components を開く
sh ./runner/rust/run.sh components --ppm > out.ppm   # ウィンドウを開かず画像を書き出す
```

初回のみ Rust（cargo）が必要です。 https://rustup.rs

## 構成

| ファイル | 役割 |
|---|---|
| `run.sh` | 引数の解釈、ビルド、起動 |
| `src/main.rs` | エントリポイント（`#![no_std]` / `#![no_main]`） |
| `src/sys.rs` | libc の最小限のラッパー（ファイル読み込み、書き出し） |
| `src/canvas.rs` | 1ピクセル単位のフレームバッファ、文字描画、PPM出力 |
| `src/font.rs` | 5x7 ASCII ビットマップフォント |
| `src/project.rs` | `config.ini` / `app/main.mb5` / `models/main.yaml` の読み取り |
| `src/render.rs` | 画面のレイアウトと描画 |
| `src/mac.rs` | macOS 用のウィンドウ表示（AppKit / CoreGraphics を objc_msgSend で直接呼ぶ） |

## 画面の仕様

- 上部: プロジェクト名（config.ini の name）と board タブ。タブをクリックすると board を切り替えます。
- 本体: 選択中の board の内容を上から順に並べたカード。ホイールまたはトラックパッドでスクロールします。
- `text`: 見出し（title）と本文（text）のカード。
- `alert`: 見出し・本文と「閉じる」ボタンのカード。閉じると非表示になります（`.display(hide)` の場合は最初から非表示）。
- `list`: 見出し・本文と、`N: "..."` の項目一覧。
- `button`: 角丸のボタン。`moveboard(名前)` を持つ場合はクリックで board を切り替えます。`.type(primary)` は塗りつぶし表示になります。

## 描画方式

- 図形（カード・ボタン・枠線）は、コードが自前のバッファに1画素ずつ書き込みます。
- 文字は macOS の OS 標準フォントで描きます（NSString の描画を、同じバッファを指す CoreGraphics のコンテキストに対して行う）。日本語も OS のフォントで表示されます。
- Retina ディスプレイでは 2 倍の解像度で描画します。
- 表示は更新のたびに CGImage を作って NSImageView に載せます。Metal / OpenGL / CoreAnimation の独自描画は使いません。
- Linux など macOS 以外では、`--ppm` で画像を書き出せます。文字は内蔵の簡易フォント（ASCII のみ）で描きます。

## 対応範囲と未対応

対応:
- `config.ini` の `name`（タイトル）
- `app/main.mb5` の `board`、`alert` / `text` / `list` / `button`、`config:title` / `config:text`、リストの項目、`moveboard()`、`.display(hide)`、`.type(primary)`
- `models/main.yaml` の、参照名に対応する `title` / `text` / `message`

未対応（仕様が未確定、または今後の実装）:
- DSL の式評価、`message()`、`wait()`、`closewelcome()` などのスクリプト実行
- 文字列の入力、キー操作
- `icon.svg` の描画
- `list(components)` のような、YAML のモデルから項目を作る機能
- 色やサイズの指定（`.bgColor` など）の反映
