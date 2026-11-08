# Mosaic Board 5 Runner (Rust)

MB5 の画面構造を読み込み、macOS 上にプレビュー表示する軽量ランナーです。外部GUIエンジンやGPU描画APIには依存せず、独自のフレームバッファとAppKit/CoreGraphicsで表示します。

## 起動

プロジェクトルート（`runner/` と `example/` がある場所）から実行します。

```sh
sh ./runner/rust/run.sh
sh ./runner/rust/run.sh path/to/components
sh ./runner/rust/run.sh components --ppm > preview.ppm
```

初回ビルドには Rust の `cargo` が必要です。macOS 以外ではウィンドウ表示ではなく `--ppm` による画像出力を使用してください。

## 表示方針

- 独自のプロジェクトヘッダーや board タブは表示しません。
- `@mainBoard` の直後に定義された `board` を初期表示します。
- `@mainBoard` がない場合は、最初に定義された `board` を初期表示します。
- `moveboard(name)` が設定されたボタンをクリックした場合は、その遷移先 board を表示します。これはDSLで定義された動作であり、ランナー独自のタブUIではありません。
- 内容がウィンドウより長い場合は、ホイール／トラックパッドでスクロールできます。

## 読み込むファイル

| ファイル | 用途 |
|---|---|
| `config.ini` | プロジェクト名などの基本設定 |
| `app/main.mb5` | board、コンポーネント、入れ子構造、修飾子 |
| `models/main.yaml` | コンポーネントの設定値・モデルデータ |
| `resources/` | アイコンなどのアセットの配置基準 |

アセットの相対パスは `resources/` を基準とします。

## MB5パーサー

パーサーは行頭の文字列検索ではなく、文字列・コメントを考慮しながら括弧とブロックを追跡します。

現在のプレビューで読み取る構文:

- `@mainBoard` と `board name { ... }`
- `alert(...)`、`text(...)`、`list(...)`、`button(...)`
- `card { ... }` 内のコンポーネント
- コンポーネント引数内に置かれた入れ子コンポーネント
- `config:title:`、`config:text:`、`config:message:`、`config:target:`
- `event: moveboard(name)`
- `.id(...)`、`.display(...)`、`.type(...)`、`.bgColor(...)`、`.fgColor(...)`、`.size(...)`、`.radius(...)`、`.width(...)` の修飾子チェーン
- `[...]` で囲まれた複数行のインラインモデル
- `models/main.yaml` の `config` と `model` の値、および `event` / `events` のブロック境界（イベント処理自体は未実装）

`[]` を付けない `model` は1行の値として扱います。`model:` のように値がない場合や、複数行データを角括弧なしで記述した場合はコンパイルエラーになります。

YAMLとMB5の両方で同一の `config` 値を指定した場合は、上書きせず競合エラーとして起動を中止します。未定義のモデル参照、コンポーネントと `target` の不一致もエラーになります。

## プレビュー上の表現

- `text`: 見出しと本文のカード
- `alert`: 見出し・本文・閉じる操作。閉じるラベルはYAMLの `model.close` があればそれを使用
- `list`: 見出し・本文と、インラインまたはYAMLのモデル項目
- `button`: ボタン。`moveboard(...)`、`.type(primary)`、色・幅・角丸・サイズの指定を反映します。`.id(...)` は識別子として保持しますが、スクリプト実行が未対応のため、現段階では実行時の参照には使用しません。
- `card { ... }`: 中のコンポーネントを解析し、表示順を維持します。現段階では独立したカードコンテナとしての装飾は行いません。

## 現段階の制限

これはMB5言語全体の完全な実行系ではなく、画面構造のプレビューランナーです。次の機能はまだ実行しません。

- `@struct` の定義・`addComponent()`／`deleteComponent()` による動的な追加・削除
- `@mainScript`、`script`、`message()`、`wait()` などのスクリプト実行
- YAMLの `events` に定義されたスクリプト呼び出し
- 入力フォームやキーボード入力
- SVGアセットの描画
- MB5全仕様を対象にした一般的なYAML 1.2の解析

未対応構文を実行できるように見せかけず、画面プレビューに必要な構造と値の解析に範囲を限定しています。
