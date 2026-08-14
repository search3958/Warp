# w3u-parser

Baram/W3U形式のXMLレイアウトをRustで読み込み、ブラウザで直接開ける単体HTMLへ変換するCLIです。

## 動作

```bash
cargo run -- path/to/layout.xml
```

リリースビルド後は:

```bash
cargo build --release
./target/release/w3u-parser path/to/layout.xml
```

入力XMLと同じフォルダへ必ず次の名前で出力します。

```text
w3u-output.html
```

例:

```text
project/
├── main.xml
└── w3u-output.html   # 生成される
```

## 対応タグ

- LinearLayout
- RelativeLayout
- FrameLayout
- GridLayout
- AbsoluteLayout
- TableLayout / TableRow
- ScrollView / HorizontalScrollView
- TextView
- EditText / AutoCompleteTextView / MultiAutoCompleteTextView
- Button / ImageButton
- CheckBox / RadioButton / RadioGroup
- ToggleButton / Switch
- Spinner / SeekBar / RatingBar
- ProgressBar
- Chronometer
- ImageView
- DatePicker / CalendarView / TimePicker / NumberPicker
- ListView / GridView / ExpandableListView
- SearchView / WebView / VideoView
- ViewFlipper / ViewAnimator / ViewSwitcher / TextSwitcher
- Space / View

未知タグは消さずに汎用`div`として保持します。

## 主な対応属性

`layout_width`, `layout_height`, `layout_weight`, `padding*`, `layout_margin*`, `background`, `alpha`, `visibility`, `enabled`, `min/maxWidth`, `min/maxHeight`, `gravity`, `layout_gravity`, GridLayout系属性、RelativeLayout系の配置属性、`text`, `textSize`, `textColor`, `textStyle`, `checked` など。

## 補足

XMLの構文解析とHTMLツリー生成はRust側で行います。生成HTMLに入るJavaScriptは、RelativeLayout/FrameLayoutの実寸依存配置と、CheckBox/Switch/ViewFlipper/Chronometer等の操作だけを担当します。
