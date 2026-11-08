# Mosaic Board 5 Runner (Rust)

Mosaic Board 5 の `components/` を読み込み、MB5の画面構成と一部のイベント・スクリプトをプレビューするランナーです。

## 起動

プロジェクトルート（`runner/` と `example/` がある場所）で実行します。

```sh
sh ./runner/rust/run.sh
sh ./runner/rust/run.sh path/to/components
sh ./runner/rust/run.sh components --ppm > preview.ppm
```

初回起動には Rust の `cargo` が必要です。フォントラスターの浮動小数点演算に使用する `libm`（`0.2`系）は、Cargoが初回ビルド時に取得します（既にキャッシュされている場合は再取得しません）。

## UIの扱い

- `@mainBoard` に対応するボードを初期表示します。画面上部に独自のヘッダーやボード切替タブは描画しません。
- `card { ... }` で明示した範囲だけをカードとして描画します。
- `list(...)` はカードではなく、通常の行リストとして描画します。各行はクリック可能で、選択状態を表示します。イベントが定義されている行をクリックすると、そのイベントを実行します。
- `button(...)` はクリック可能です。`event: moveboard(boardName)` が指定されている場合は、そのボードへ移動します。
- `alert(...)` はカードとして配置せず、暗いバックドロップを伴うモーダルダイアログとして表示します。閉じる操作はYAMLの `events.close` または `model.close.event` に接続できます。
- カード内の連続したボタンは横並びに配置し、幅が足りない場合は折り返します。
- ホイール／トラックパッドでコンテンツをスクロールできます。オーバーフロー時は右端に細いスクロールバーを表示します。
- ボード切り替え用の独自タブやコンテンツ内ヘッダーは表示しません。

## MB5の読み取り範囲

現在のランナーが扱う主な構文は次のとおりです。

- `@mainBoard`、`board`、`card { ... }`
- `text(...)`、`list(...)`、`button(...)`、`alert(...)`
- `.id()`、`.display()`、`.type()`、`.width()`、`.size()`、`.radius()`、`.bgColor()`、`.fgColor()`
  - `.type(primary)`: アクセント色の塗りボタン（白文字）
  - `.type(text)`: 背景を持たないテキストボタン（アクセント色文字）
  - `.type(...)` を指定しない通常ボタン: 薄いニュートラル面（アクセント色文字）
- コンポーネント内の `config:title:`、`config:text:`、`config:message:`、`event: moveboard(...)`
- YAMLの `config`、`model`、`event`／`events`
- `@struct` の `components name { ... }` と `addComponent()`／`deleteComponent()`
- `@mainScript` のスクリプト、`message(...)` 分岐、`wait()`、スクリプト呼び出し、`setComponentConfig()`
- `.target(boardName)` による命令対象ボードの指定

YAMLの `#` コメントは解析前に除外します。コメント開始は引用符の外側で、行頭または空白の後にある `#` です。ダブルクォート内のエスケープ、シングルクォート内の `''`、文字列中の `#` は保持します。YAMLの `//` はコメント記号として扱いません。

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

## 表示フォント・アンチエイリアス

`assets/HarmonyOS_Sans_SC_Regular.ttf` を内蔵し、vendored `stb_truetype_rust` でグリフをソフトウェアラスター化します。macOSのAppKit／CoreText、WindowsのDirectWrite、Linuxのフォントサービスを使わず、macOS・Linux・Windowsの対応ターゲットで同じフォントデータ・メトリクス・ピクセル描画を使用します。

グリフのカバレッジは8bitアルファとしてフレームバッファへ合成します。文字の計測も同じTTFメトリクスを使うため、プレビューと折り返し計算の差を抑えます。グリフ画像は64KiBの固定キャッシュ領域と4KiBの作業領域に保持し、OSのフォントAPIやシステムフォント登録に依存しません。見出しは24px相当、説明文は13px、リスト項目は14px、ボタンは18px相当を基本にします。Retinaなどの高密度画面ではデバイス倍率に合わせたサイズでグリフを再ラスター化します。

角丸矩形は4段の垂直サブピクセルでサンプリングし、各サブ行の水平エッジについてピクセルとの重なりを連続値で計算して8bitアルファ合成します。直線部分は直接塗り、角の領域だけをサンプリングするため、全領域を高倍率描画する方法より軽量です。

## UIデザイン基準

見た目は `baram-warp4` の通常（ライト）プロファイルを参考にしています。

| 用途 | 色 |
|---|---|
| 画面背景 | `#FAFAFC` |
| カード・行の面 | `#FBFBFB` |
| 本文 | `#1A1A1A` |
| 補助テキスト | `#5D5D5D` |
| 境界線 | `#D3D3D3` |
| アクセント | `#006AFF` |
| 通常ボタン面 | `#EEEEEF` |

通常ボタンはニュートラルな面にアクセント色の文字、Primaryボタンはアクセント色の面に白文字、Textボタンは背景なしのアクセント色文字です。通常のボタンはカプセル形状を基本とし、`.radius(...)` が指定された場合はその値を使用します。リストは外枠のカードにせず、フラットな行と区切り線で表示します。

## 描画・スクロールの実装方式

- 図形・角丸・テキストはOS描画APIを使わず、自前のフレームバッファへ描画します。
- スクロール位置、範囲制限、クリッピング、スクロールバーは `App`／`Canvas` が管理し、ネイティブのScrollViewには依存しません。
- ウィンドウバックエンドは入力イベントを論理ピクセルへ正規化し、共通の `App::scroll()` に渡すだけです。現在同梱している対話型ウィンドウバックエンドはmacOS用ですが、スクロールの状態管理・描画処理自体はOS非依存です。
- `--ppm` はmacOS以外でも同じTTF・アンチエイリアス・角丸描画でPPMを出力します（画像出力モードのため対話スクロールはありません）。

## 現時点の制限

このランナーはMB5全仕様のコンパイラ／実行環境ではなく、サンプルを確認するためのプレビュー実装です。現時点では、任意の式評価、全コンポーネント・全修飾子、アイコン画像の描画、入力フォーム、完全な診断メッセージ、すべてのネスト構文は未対応です。未対応構文を使う場合は、このプレビュー結果を正式なMB5実行結果として扱わないでください。
