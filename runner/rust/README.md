# MosaicBoard 5 Runner

MB5 の宣言的UI、YAMLモデル、Picol風スクリプトを読み込む小型プレビューランナーです。描画とレイアウトはRustの固定長データ構造・ソフトウェアCanvasを中心に構成し、スクリプトVMは独立したC99モジュールです。

## Quick start

プロジェクトルートで実行します。

```sh
sh ./runner/rust/run.sh
sh ./runner/rust/run.sh 50%
sh ./runner/rust/run.sh ./example/components
sh ./runner/rust/run.sh ./example/components --ppm > preview.ppm
```

`cargo` とRustツールチェーンが必要です。インタラクティブウィンドウは現在macOSバックエンド、`--ppm` はオフスクリーン画像出力です。 `50%` などの指定はCPUクロックではなく、描画上限と旧式スクリプトの実行予算・待機時間を調整します。`100%` は最大60FPSです。新しいC VMの手続きや `fetch` を厳密にスロットリングするものではありません。

## Project files

```text
components/
  config.ini
  app/main.mb5       # UI構造
  models/main.yaml   # list / navigation / dropdown / tab のモデル
  scripts/main.mb5   # script procedures and events
  resources/         # image and other assets
```

別ファイルの `scripts/main.mb5` があれば、旧 `@mainScript` セクションより優先されます。ファイルは上から順に評価するため、`proc` 定義より前に `set` / `let` などの初期化処理も書けます。`proc main {} { ... }` があればUI構築後に呼び出します。

## Current UI foundation

- `icon`, `input`, `checkbox`, `switch`, `slider`, `progress`, `divider`
- `scrollbox`, `topbar`, `navigation`, `dropdown`, `tab`
- `spacing`, `horizontal`, `vertical`, `grid`, `snackbar`
- `gradient`, `opacity`, `.transition(milliseconds)`
- existing `text`, `list`, `button`, `card`, `alert`

`topbar` はboard直下に1つだけ定義し、ランタイムが位置を決めます。`.transition(ms)` の対象は色・グラデーション・透明度のみで、位置・サイズ・形状はアニメーションしません。

Primary color は `config.ini` の `primaryColor=#006AFF` で指定できます。その他の見た目は標準テーマを優先する設計です。

## Script foundation

スクリプトはPicol/Tcl風のコマンド構文です。`proc`, `set`, `expr`, `if`, `while`, `foreach`, `list`, `dict`, `string`, `catch`, `json.get`, `ui.get`, `ui.set`, `after`, `fetch` などを実装しています。

`fetch` は GET/POST/PUT/PATCH/DELETE に対応します。システムの `curl` を使い、HTTP(S)限定、接続5秒・全体15秒、本文4KiB、応答16KiBの上限があります。HTTPエラー応答も `HTTP_STATUS` と本文を返します。**現在のfetchは同期処理のため、通信中はUIイベントループを止めます。**

`after milliseconds procedure` はイベントループ上の非ブロッキングタイマーです。UI処理では旧 `wait` より `after` を使用してください。

## Specification and development

- [MB5 language and runtime specification](../../docs/MB5_SPEC.md)
- [Development guide](../../docs/DEVELOPMENT.md)
- [VS Code language support](../../tools/vscode/README.md)

Validation:

```sh
sh ./tools/mb5/check.sh
```

Rust build は `cargo check` または `run.sh` で行います。`check.sh` はC VMの単体テストとスクリプト構文検査も実行します。

## Known limitations

このZIPは動作基礎と開発用土台です。完全なMB5コンパイラではありません。スクリプト値は現状文字列ベース、JSONのUnicode `\uXXXX` はデコードせず保持、fetchは同期、import/module、async I/O、厳密な静的型、カスタムモーションDSLは未実装です。スクリプトVMとUIホストは単一スレッド・単一Appの実行を前提とします。
