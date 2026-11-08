# Mosaic Board 5 Runner (Rust)

Mosaic Board 5 の `components/` を読み込み、MB5の画面構成と一部のイベント・スクリプトをプレビューするランナーです。

## 起動

プロジェクトルート（`runner/` と `example/` がある場所）で実行します。

```sh
sh ./runner/rust/run.sh
sh ./runner/rust/run.sh path/to/components
sh ./runner/rust/run.sh components --ppm > preview.ppm
```

初回起動には Rust の `cargo` が必要です。

## UIの扱い

- `@mainBoard` に対応するボードを初期表示します。画面上部に独自のヘッダーやボード切替タブは描画しません。
- `card { ... }` で明示した範囲だけをカードとして描画します。
- `list(...)` はカードではなく、通常の行リストとして描画します。各行はクリック可能で、選択状態を表示します。イベントが定義されている行をクリックすると、そのイベントを実行します。
- `button(...)` はクリック可能です。`event: moveboard(boardName)` が指定されている場合は、そのボードへ移動します。
- `alert(...)` はカードとして配置せず、暗いバックドロップを伴うモーダルダイアログとして表示します。閉じる操作はYAMLの `events.close` または `model.close.event` に接続できます。
- カード内の連続したボタンは横並びに配置し、幅が足りない場合は折り返します。
- ホイール／トラックパッドでコンテンツをスクロールできます。

## MB5の読み取り範囲

現在のランナーが扱う主な構文は次のとおりです。

- `@mainBoard`、`board`、`card { ... }`
- `text(...)`、`list(...)`、`button(...)`、`alert(...)`
- `.id()`、`.display()`、`.type()`、`.width()`、`.size()`、`.radius()`、`.bgColor()`、`.fgColor()`
- コンポーネント内の `config:title:`、`config:text:`、`config:message:`、`event: moveboard(...)`
- YAMLの `config`、`model`、`event`／`events`
- `@struct` の `components name { ... }` と `addComponent()`／`deleteComponent()`
- `@mainScript` のスクリプト、`message(...)` 分岐、`wait()`、スクリプト呼び出し、`setComponentConfig()`
- `.target(boardName)` による命令対象ボードの指定

YAMLの `model` 内に定義したIDはリスト項目などのデータ識別子として扱います。`events` の同名IDは、そのデータ項目に紐付けられます。モデル項目のオブジェクト形式では `text` と `event` を読み取ります。

### スクリプトの実行

`@mainScript` 内で最初に定義されたスクリプトを起動時に実行します。例えば、次の処理は1秒後に `closewelcome()` を呼び出します。

```text
@mainScript
script hello{
    wait(1000)
    closewelcome()
}

script closewelcome{
    setComponentConfig(welcome-alert,display:hide)
}
```

`wait()` 後の命令は、その命令が実行される時点の現在のボードを基準にします。`.target()` が指定されている場合は、そのボードが対象です。

### リストのイベント例

```yaml
components:
  model:
    easteregg:
      text: "Lucky board"
      event: luckyalert(show)
```

または、IDと表示文字列を `model` に置き、複数イベントを `events` にまとめて定義できます。

```yaml
components:
  events:
    easteregg: luckyalert(show)
  model:
    easteregg: "Lucky board"
```

## 表示フォント

macOSでは `HarmonyOS Sans SC` がシステムにインストールされていれば通常テキストに使用します。フォントが見つからない場合はシステムフォントへフォールバックします。見出し・ボタンは太字スタイルを使用します。

フォントファイルはこのソース配布物には含めていません。指定フォントを使う場合は、ユーザー自身のフォントファイルをmacOSのFont Bookからインストールしてください。

## 描画・実装方式

- 図形は自前のフレームバッファへ描画します。
- macOSの文字描画はAppKit／CoreGraphicsを使用します。
- macOSではRetina倍率に合わせて描画します。
- `--ppm` は描画結果をPPMとして出力します。macOS以外では簡易ASCIIフォントを使用します。

## 現時点の制限

このランナーはMB5全仕様のコンパイラ／実行環境ではなく、サンプルを確認するためのプレビュー実装です。現時点では、任意の式評価、全コンポーネント・全修飾子、アイコン画像の描画、入力フォーム、完全な診断メッセージ、すべてのネスト構文は未対応です。未対応構文を使う場合は、このプレビュー結果を正式なMB5実行結果として扱わないでください。
