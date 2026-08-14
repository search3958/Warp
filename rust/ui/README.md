# Warp 4 UI 仕様書

## 基本

Warp 4 UIはXML形式で画面を記述するUIシステムです。

・XML 1.0を使用します
・文字コードはUTF-8を使用します
・1ファイルにつきルート要素は必ず1個です
・タグ名，属性名は大文字小文字を区別します
・基本的にAndroid XML Layoutに近い記述方法を使用します
・Warp 4 ScriptからIDを指定して動的に変更できます
・UIファイル単体では画面構造と初期状態のみを定義します
・クリック後の処理などはWarp 4 Script側から定義できます

### 名前空間

```xml
xmlns:baram="http://schemas.baram.com/apk/res/baram"
```

基本的にルート要素で定義します。

```xml
<LinearLayout
    xmlns:baram="http://schemas.baram.com/apk/res/baram"
    baram:layout_width="match_parent"
    baram:layout_height="match_parent">
</LinearLayout>
```

・`baram:` はWarp 4 UI属性を表します
・名前空間URIは `http://schemas.baram.com/apk/res/baram` です
・名前空間を認識するパーサーでは、接頭辞そのものではなくURIによって判定します
・そのため技術上は別の接頭辞を使用できますが、標準では必ず `baram:` を推奨します

---

# 値

## 文字列

```xml
baram:text="Hello"
```

・`"` で囲みます
・XML特殊文字はXMLエスケープが必要です

```xml
baram:text="A &amp; B"
```

主な特殊文字：

```text
&   → &amp;
<   → &lt;
>   → &gt;
"   → &quot;
```

---

## 真偽値

```text
true
false
```

例：

```xml
baram:checked="true"
```

・大文字小文字は区別しない実装でも構いません
・標準表記は小文字の `true` / `false` です
・値が不正な場合、原則として `false` として扱います
・ただし `enabled` など初期値がtrueの属性は、その属性が存在しない場合はtrueです

---

## サイズ

### match_parent

```xml
baram:layout_width="match_parent"
```

・親が利用可能な領域いっぱいまで広がります

### fill_parent

```xml
baram:layout_width="fill_parent"
```

・`match_parent` と完全に同じ意味です
・互換性のために存在します
・新規コードでは `match_parent` を推奨します

### wrap_content

```xml
baram:layout_width="wrap_content"
```

・内容を表示するために必要な最小サイズになります

### 固定値

```xml
baram:layout_width="120dp"
baram:layout_height="48dp"
```

使用可能単位：

```text
dp
dip
sp
px
```

---

## dp

```text
12dp
48dp
120dp
```

UI部品や余白など通常のサイズに使用します。

---

## dip

```text
12dip
```

・`dp` と同一です
・互換表記としてのみ存在します

---

## sp

```text
18sp
```

主に文字サイズに使用します。

```xml
baram:textSize="18sp"
```

---

## px

```text
100px
```

物理ピクセル相当の固定値として扱います。

---

## 数値

一部の属性では単位なしの数値を使用します。

```xml
baram:layout_weight="1"
baram:max="100"
baram:progress="50"
```

・整数と小数を使用できます
・数値変換できない場合は属性ごとの初期値を使用します

---

## 色

基本：

```text
#RGB
#RRGGBB
```

例：

```xml
baram:textColor="#ffffff"
baram:background="#202020"
```

実装が対応している場合：

```text
rgb(...)
rgba(...)
```

も使用できます。

---

# ID

## IDの定義

```xml
baram:id="@+id/title"
```

`@+id/名前` によりIDを定義します。

例：

```xml
<TextView
    baram:id="@+id/title"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:text="Hello"/>
```

---

## IDの参照

```text
@id/名前
```

または互換形式：

```text
@+id/名前
@baram:id/名前
```

例：

```xml
baram:layout_below="@id/title"
```

ID比較時には以下はすべて `title` として扱います。

```text
@+id/title
@id/title
@baram:id/title
title
```

---

## ID名

推奨：

```text
title
main
submitButton
user_name
item1
```

・同一画面内で重複してはいけません
・Warp 4 ScriptからアクセスするときはID名部分だけを使用します

```text
WarpUI.text title (Hello)
```

---

# 共通属性

ほぼすべてのViewで使用可能です。

## layout_width

```xml
baram:layout_width="match_parent"
```

Viewの横幅です。

指定可能：

```text
match_parent
fill_parent
wrap_content
0dp
固定値
```

---

## layout_height

```xml
baram:layout_height="wrap_content"
```

Viewの高さです。

---

## layout_weight

```xml
baram:layout_weight="1"
```

主にLinearLayoutで使用します。

例：

```xml
<View
    baram:layout_width="0dp"
    baram:layout_height="40dp"
    baram:layout_weight="1"/>
```

親がhorizontalの場合：

・`layout_width="0dp"` と組み合わせると横方向にweight配分されます

親がverticalの場合：

・`layout_height="0dp"` と組み合わせると縦方向にweight配分されます

複数存在する場合：

```xml
<View baram:layout_weight="1"/>
<View baram:layout_weight="2"/>
```

利用可能領域は概ね

```text
1 : 2
```

で分配されます。

---

# Padding

## padding

```xml
baram:padding="12dp"
```

上下左右すべてに適用します。

## paddingHorizontal

```xml
baram:paddingHorizontal="12dp"
```

左右に適用します。

## paddingVertical

```xml
baram:paddingVertical="12dp"
```

上下に適用します。

## 個別指定

```xml
baram:paddingTop="10dp"
baram:paddingBottom="10dp"
baram:paddingLeft="12dp"
baram:paddingRight="12dp"
```

Start / Endも使用可能です。

```xml
baram:paddingStart="12dp"
baram:paddingEnd="12dp"
```

### 優先順位

個別指定が最優先です。

```text
paddingLeft
↓
paddingStart
↓
paddingHorizontal
↓
padding
```

同様に各方向に適用されます。

---

# Margin

## layout_margin

```xml
baram:layout_margin="12dp"
```

## layout_marginHorizontal

```xml
baram:layout_marginHorizontal="12dp"
```

## layout_marginVertical

```xml
baram:layout_marginVertical="8dp"
```

## 個別指定

```xml
baram:layout_marginTop="8dp"
baram:layout_marginBottom="8dp"
baram:layout_marginLeft="12dp"
baram:layout_marginRight="12dp"
baram:layout_marginStart="12dp"
baram:layout_marginEnd="12dp"
```

---

# 背景

```xml
baram:background="#ffffff"
```

Viewの背景を設定します。

---

# 透明度

```xml
baram:alpha="0.5"
```

範囲：

```text
0.0 ～ 1.0
```

```text
0 = 完全透明
1 = 完全不透明
```

属性なしの場合：

```text
1
```

---

# 表示状態

```xml
baram:visibility="visible"
```

使用可能：

```text
visible
invisible
gone
```

## visible

通常表示します。

## invisible

表示しませんが、レイアウト上の領域は残します。

## gone

表示せず、原則としてレイアウト上の領域も使用しません。

---

# enabled

```xml
baram:enabled="false"
```

`false` の場合：

・操作不能
・クリック不能
・入力不能
・無効状態として表示

属性なしの場合：

```text
true
```

---

# 最小・最大サイズ

```xml
baram:minWidth="100dp"
baram:minHeight="40dp"

baram:maxWidth="500dp"
baram:maxHeight="300dp"
```

---

# gravity

View内部の内容の配置です。

```xml
baram:gravity="center"
```

使用可能：

```text
left
right
start
end
top
bottom
center
center_horizontal
center_vertical
```

複数指定：

```xml
baram:gravity="center_vertical|right"
```

---

# layout_gravity

親の中でView自身をどこに配置するかを指定します。

```xml
baram:layout_gravity="center"
```

`gravity` と `layout_gravity` は別物です。

```text
gravity
→ 自分の中身

layout_gravity
→ 自分自身
```

---

# Layout

## LinearLayout

```xml
<LinearLayout
    baram:layout_width="match_parent"
    baram:layout_height="match_parent"
    baram:orientation="vertical">

</LinearLayout>
```

子Viewを一方向に並べます。

---

## orientation

### vertical

```xml
baram:orientation="vertical"
```

上から下へ配置します。

### horizontal

```xml
baram:orientation="horizontal"
```

左から右へ配置します。

属性なしの場合：

```text
vertical
```

---

## LinearLayoutの例

```xml
<LinearLayout
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"
    baram:orientation="horizontal">

    <Button
        baram:layout_width="wrap_content"
        baram:layout_height="wrap_content"
        baram:text="A"/>

    <Button
        baram:layout_width="wrap_content"
        baram:layout_height="wrap_content"
        baram:text="B"/>

</LinearLayout>
```

---

# RelativeLayout

```xml
<RelativeLayout
    baram:layout_width="match_parent"
    baram:layout_height="200dp">

</RelativeLayout>
```

他のViewまたは親を基準にして位置を指定できます。

---

## 親基準

### 左

```xml
baram:layout_alignParentLeft="true"
```

### 右

```xml
baram:layout_alignParentRight="true"
```

### Start

```xml
baram:layout_alignParentStart="true"
```

### End

```xml
baram:layout_alignParentEnd="true"
```

### 下

```xml
baram:layout_alignParentBottom="true"
```

### 横中央

```xml
baram:layout_centerHorizontal="true"
```

### 縦中央

```xml
baram:layout_centerVertical="true"
```

### 完全中央

```xml
baram:layout_centerInParent="true"
```

---

## 他View基準

### 下

```xml
baram:layout_below="@id/title"
```

### 上

```xml
baram:layout_above="@id/title"
```

### 右

```xml
baram:layout_toRightOf="@id/title"
```

### End

```xml
baram:layout_toEndOf="@id/title"
```

### 左

```xml
baram:layout_toLeftOf="@id/title"
```

### Start

```xml
baram:layout_toStartOf="@id/title"
```

---

## 位置合わせ

```xml
baram:layout_alignLeft="@id/title"
baram:layout_alignRight="@id/title"
baram:layout_alignTop="@id/title"
baram:layout_alignBottom="@id/title"
```

Start / End：

```xml
baram:layout_alignStart="@id/title"
baram:layout_alignEnd="@id/title"
```

---

# FrameLayout

```xml
<FrameLayout
    baram:layout_width="match_parent"
    baram:layout_height="200dp">
</FrameLayout>
```

子Viewを重ねて表示します。

基本的にはすべて同じ領域に配置されます。

子Viewの：

```xml
baram:layout_gravity
```

によって位置を変更できます。

例：

```xml
<TextView
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:layout_gravity="center"
    baram:text="Center"/>
```

---

# ScrollView

```xml
<ScrollView
    baram:layout_width="match_parent"
    baram:layout_height="match_parent">

    <LinearLayout
        baram:layout_width="match_parent"
        baram:layout_height="wrap_content"
        baram:orientation="vertical">

    </LinearLayout>

</ScrollView>
```

縦方向スクロールを行います。

・原則として直接の子Viewは1個です
・複数要素を入れたい場合はLinearLayoutなどを1個入れます

---

## fillViewport

```xml
baram:fillViewport="true"
```

子Viewが小さい場合でもScrollViewの表示領域まで拡張できるようにします。

---

# HorizontalScrollView

```xml
<HorizontalScrollView
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content">
</HorizontalScrollView>
```

横方向スクロールです。

・直接の子は原則1個です

---

# GridLayout

```xml
<GridLayout
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"
    baram:columnCount="3">
</GridLayout>
```

グリッド状に配置します。

---

## columnCount

```xml
baram:columnCount="3"
```

列数です。

最低：

```text
1
```

---

## layout_column

```xml
baram:layout_column="1"
```

列位置を指定します。

0から開始します。

---

## layout_row

```xml
baram:layout_row="2"
```

行位置を指定します。

0から開始します。

---

## layout_columnSpan

```xml
baram:layout_columnSpan="2"
```

複数列を使用します。

---

## layout_rowSpan

```xml
baram:layout_rowSpan="2"
```

複数行を使用します。

---

# TableLayout

```xml
<TableLayout
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content">
</TableLayout>
```

TableRowを子に持ちます。

---

## TableRow

```xml
<TableRow>
    <TextView
        baram:layout_width="wrap_content"
        baram:layout_height="wrap_content"
        baram:text="A"/>

    <TextView
        baram:layout_width="wrap_content"
        baram:layout_height="wrap_content"
        baram:text="B"/>
</TableRow>
```

1つのTableRowが1行です。

---

## stretchColumns

```xml
baram:stretchColumns="*"
```

`*` の場合、すべての列を利用可能幅まで拡張します。

---

# AbsoluteLayout

```xml
<AbsoluteLayout
    baram:layout_width="match_parent"
    baram:layout_height="200dp">
</AbsoluteLayout>
```

絶対位置による配置です。

---

## layout_x

```xml
baram:layout_x="100dp"
```

左からの位置。

## layout_y

```xml
baram:layout_y="50dp"
```

上からの位置。

例：

```xml
<Button
    baram:layout_width="120dp"
    baram:layout_height="wrap_content"
    baram:layout_x="50dp"
    baram:layout_y="20dp"
    baram:text="Button"/>
```

・互換性目的で利用できます
・通常はLinearLayout / RelativeLayout等を推奨します

---

# View

最小の描画要素です。

```xml
<View
    baram:layout_width="match_parent"
    baram:layout_height="1dp"
    baram:background="#808080"/>
```

区切り線などに使用できます。

---

# Space

```xml
<Space
    baram:layout_width="match_parent"
    baram:layout_height="20dp"/>
```

何も表示しない空白Viewです。

---

# TextView

```xml
<TextView
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:text="Hello"/>
```

文字を表示します。

---

## text

```xml
baram:text="Hello"
```

表示文字列です。

---

## textSize

```xml
baram:textSize="18sp"
```

---

## textColor

```xml
baram:textColor="#000000"
```

---

## textStyle

```xml
baram:textStyle="bold"
```

使用可能：

```text
bold
italic
```

組み合わせ：

```xml
baram:textStyle="bold|italic"
```

---

## maxLines

```xml
baram:maxLines="2"
```

最大表示行数です。

---

# EditText

```xml
<EditText
    baram:id="@+id/input"
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"
    baram:hint="Input"/>
```

文字入力欄です。

---

## hint

```xml
baram:hint="Type something..."
```

入力されていない場合の表示です。

---

## text

```xml
baram:text="Default"
```

初期値です。

---

## singleLine

```xml
baram:singleLine="true"
```

1行入力にします。

`false` の場合は複数行入力として扱えます。

---

# AutoCompleteTextView

```xml
<AutoCompleteTextView
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"
    baram:hint="Search"/>
```

EditText互換の入力Viewです。

候補データそのものはUI XMLではなくアプリ側から与えます。

---

# MultiAutoCompleteTextView

```xml
<MultiAutoCompleteTextView
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"/>
```

複数候補入力用です。

---

# Button

```xml
<Button
    baram:id="@+id/button"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:text="OK"/>
```

通常のボタンです。

クリック処理はWarp 4 Scriptから定義できます。

```text
WarpUI.OnClick button {
    print(Clicked)
}
```

---

# ImageButton

```xml
<ImageButton
    baram:id="@+id/camera"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:src="@baram:drawable/ic_menu_camera"
    baram:contentDescription="Camera"/>
```

画像付きボタンです。

---

## src

```xml
baram:src="..."
```

表示する画像リソースです。

---

## contentDescription

```xml
baram:contentDescription="Camera"
```

画像の説明です。

---

# ImageView

```xml
<ImageView
    baram:layout_width="100dp"
    baram:layout_height="100dp"
    baram:src="image.png"/>
```

画像表示用Viewです。

---

# CheckBox

```xml
<CheckBox
    baram:id="@+id/check"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:checked="true"
    baram:text="Enable"/>
```

---

## checked

```xml
baram:checked="true"
```

初期状態です。

---

## 操作

クリックするたびに：

```text
true → false
false → true
```

へ変更されます。

---

# RadioGroup

```xml
<RadioGroup
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:orientation="vertical">

</RadioGroup>
```

RadioButtonをまとめます。

同じRadioGroup直下に存在するRadioButtonは原則として1個だけ選択可能です。

---

# RadioButton

```xml
<RadioButton
    baram:id="@+id/a"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:text="A"
    baram:checked="true"/>
```

選択すると同一RadioGroup内の他のRadioButtonは解除されます。

---

# ToggleButton

```xml
<ToggleButton
    baram:id="@+id/toggle"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:checked="true"/>
```

---

## textOn

```xml
baram:textOn="ON"
```

## textOff

```xml
baram:textOff="OFF"
```

省略時：

```text
ON
OFF
```

---

# Switch

```xml
<Switch
    baram:id="@+id/switch"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:text="Wi-Fi"
    baram:checked="true"/>
```

クリックするごとにchecked状態が切り替わります。

---

# Spinner

```xml
<Spinner
    baram:id="@+id/spinner"
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"/>
```

選択リストです。

選択肢の内容は原則としてアプリ側から設定します。

UIのみのプレビュー環境では仮データを表示しても構いません。

---

# SeekBar

```xml
<SeekBar
    baram:id="@+id/seek"
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"
    baram:max="100"
    baram:progress="50"/>
```

---

## max

```xml
baram:max="100"
```

最大値です。

既定値：

```text
100
```

---

## progress

```xml
baram:progress="50"
```

初期値です。

既定値：

```text
0
```

---

## min

```xml
baram:min="0"
```

最小値です。

既定値：

```text
0
```

---

# RatingBar

```xml
<RatingBar
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:numStars="5"
    baram:rating="3.5"
    baram:stepSize="0.5"/>
```

---

## numStars

星の個数です。

```xml
baram:numStars="5"
```

既定値：

```text
5
```

---

## rating

```xml
baram:rating="3.5"
```

現在値。

---

## stepSize

```xml
baram:stepSize="0.5"
```

1回の変更単位。

---

# ProgressBar

## 通常

```xml
<ProgressBar
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"/>
```

回転式の進捗表示です。

---

## Horizontal

```xml
<ProgressBar
    style="?baram:attr/progressBarStyleHorizontal"
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"
    baram:max="100"
    baram:progress="65"/>
```

横方向の進捗バーになります。

---

# Chronometer

```xml
<Chronometer
    baram:id="@+id/timer"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"/>
```

経過時間を表示します。

基本表示：

```text
00:00
```

実行開始後に増加します。

---

# DatePicker

```xml
<DatePicker
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"/>
```

日付入力UIです。

---

# CalendarView

```xml
<CalendarView
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"/>
```

日付選択UIとして扱います。

---

# TimePicker

```xml
<TimePicker
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"/>
```

時刻入力UIです。

---

# NumberPicker

```xml
<NumberPicker
    baram:id="@+id/count"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:minValue="0"
    baram:maxValue="100"
    baram:value="10"/>
```

---

## minValue

```xml
baram:minValue="0"
```

---

## maxValue

```xml
baram:maxValue="100"
```

---

## value

```xml
baram:value="10"
```

初期値です。

---

# ListView

```xml
<ListView
    baram:id="@+id/list"
    baram:layout_width="match_parent"
    baram:layout_height="match_parent"/>
```

縦方向リストです。

リストデータはUIファイルではなくアプリ側から設定します。

---

# GridView

```xml
<GridView
    baram:id="@+id/grid"
    baram:layout_width="match_parent"
    baram:layout_height="match_parent"
    baram:numColumns="3"/>
```

---

## numColumns

```xml
baram:numColumns="3"
```

列数です。

最低1。

---

# ExpandableListView

```xml
<ExpandableListView
    baram:id="@+id/list"
    baram:layout_width="match_parent"
    baram:layout_height="match_parent"/>
```

階層型リストです。

実際のGroup / Childデータはアプリ側から設定します。

---

# SearchView

```xml
<SearchView
    baram:id="@+id/search"
    baram:layout_width="match_parent"
    baram:layout_height="wrap_content"
    baram:queryHint="Search"/>
```

---

## queryHint

```xml
baram:queryHint="Search..."
```

検索欄のヒントです。

---

# WebView

```xml
<WebView
    baram:id="@+id/web"
    baram:layout_width="match_parent"
    baram:layout_height="match_parent"/>
```

Webコンテンツを表示する領域です。

URLなどの読み込みはWarp 4 Scriptまたはアプリ側で行います。

---

# VideoView

```xml
<VideoView
    baram:id="@+id/video"
    baram:layout_width="match_parent"
    baram:layout_height="200dp"/>
```

動画表示領域です。

再生する動画はアプリ側から指定します。

---

# ViewFlipper

```xml
<ViewFlipper
    baram:id="@+id/flipper"
    baram:layout_width="match_parent"
    baram:layout_height="100dp"
    baram:autoStart="true"
    baram:flipInterval="2000">

    <TextView
        baram:layout_width="match_parent"
        baram:layout_height="match_parent"
        baram:text="1"/>

    <TextView
        baram:layout_width="match_parent"
        baram:layout_height="match_parent"
        baram:text="2"/>

</ViewFlipper>
```

複数の子Viewから1個だけを表示します。

---

## autoStart

```xml
baram:autoStart="true"
```

自動切り替えを開始します。

既定値：

```text
false
```

---

## flipInterval

```xml
baram:flipInterval="2000"
```

ミリ秒単位です。

```text
2000 = 2秒
```

---

# ViewAnimator

```xml
<ViewAnimator>
</ViewAnimator>
```

ViewFlipper互換の複数画面Viewとして扱います。

---

# ViewSwitcher

```xml
<ViewSwitcher>
</ViewSwitcher>
```

複数View切り替え用です。

---

# TextSwitcher

```xml
<TextSwitcher>
</TextSwitcher>
```

文字View切り替え用です。

---

# Style

```xml
style="@style/ComponentBox"
```

`style` は `baram:` を付けません。

例：

```xml
<TextView
    style="@style/SectionTitle"
    baram:text="Settings"/>
```

標準スタイルとして以下を使用できます。

```text
@style/SectionTitle
@style/SectionDescription
@style/ComponentLabel
@style/ComponentBox
```

---

# SectionTitle

大きなセクションタイトル用です。

```xml
<TextView
    style="@style/SectionTitle"
    baram:text="Buttons"/>
```

---

# SectionDescription

説明文用。

```xml
<TextView
    style="@style/SectionDescription"
    baram:text="Button components"/>
```

---

# ComponentLabel

コンポーネント名表示用。

```xml
<TextView
    style="@style/ComponentLabel"
    baram:text="Button"/>
```

---

# ComponentBox

コンポーネントの標準表示領域用。

```xml
<Button
    style="@style/ComponentBox"
    baram:text="Button"/>
```

---

# 属性の解決

Warp 4 UIでは属性を以下の順に解決します。

```text
1. 要素に直接指定された属性
2. styleによって設定された値
3. Widget固有の既定値
4. Warp 4 UI共通既定値
```

直接指定された属性が最優先です。

例：

```xml
<TextView
    style="@style/SectionTitle"
    baram:textSize="12sp"/>
```

この場合、SectionTitle側のtextSizeより

```text
12sp
```

が優先されます。

---

# 不明な属性

例：

```xml
baram:somethingUnknown="123"
```

実装が知らない属性は原則として無視します。

・XML全体の解析を停止してはいけません
・将来の属性との互換性を維持するためです

---

# 不明なタグ

例：

```xml
<MyFutureWidget>
</MyFutureWidget>
```

互換モードでは空のViewとして扱うことができます。

厳密モードでは：

```text
Unknown Widget: MyFutureWidget
```

としてエラーにできます。

標準実行環境では互換性を優先し、可能な限り画面全体の描画を継続することを推奨します。

---

# XMLエラー

以下はUIとして実行できません。

```xml
<LinearLayout>
    <TextView>
</LinearLayout>
```

タグが閉じられていないためXML構文エラーです。

XMLとして解析できない場合：

・画面構築を開始しない
・可能であればXML解析エラーを表示する
・途中までのUIを実行してはいけません

---

# レイアウト計算

基本的な計算順序：

```text
XML解析
↓
Viewツリー生成
↓
style適用
↓
layout_width / layout_height
↓
padding / margin
↓
親レイアウト規則
↓
RelativeLayout等の位置計算
↓
描画
```

---

# LinearLayoutのweight

vertical：

```xml
<LinearLayout
    baram:orientation="vertical">

    <View
        baram:layout_width="match_parent"
        baram:layout_height="0dp"
        baram:layout_weight="1"/>

    <View
        baram:layout_width="match_parent"
        baram:layout_height="0dp"
        baram:layout_weight="2"/>

</LinearLayout>
```

残り高さを

```text
1 : 2
```

に分割します。

horizontalの場合はwidth方向に適用されます。

---

# RelativeLayoutの解決

Sibling IDを参照する属性：

```text
layout_below
layout_above
layout_toRightOf
layout_toLeftOf
layout_toStartOf
layout_toEndOf
layout_alignLeft
layout_alignRight
layout_alignStart
layout_alignEnd
layout_alignTop
layout_alignBottom
```

対象IDが存在しない場合：

・その指定のみ無視します
・画面全体をエラーにはしません

---

# 初期値

明示されていない場合の主要初期値：

```text
visibility = visible
enabled = true
alpha = 1
orientation = vertical
checked = false
layout_weight = 0
SeekBar min = 0
SeekBar max = 100
SeekBar progress = 0
RatingBar numStars = 5
ViewFlipper autoStart = false
ViewFlipper flipInterval = 3000
```

---

# Warp 4 Scriptとの接続

Warp 4 ScriptからUIを操作する場合、XMLで定義したIDを使用します。

例：

```xml
<TextView
    baram:id="@+id/title"
    baram:layout_width="wrap_content"
    baram:layout_height="wrap_content"
    baram:text="Hello"/>
```

Script：

```text
WarpUI.text title (World)
```

結果：

```text
Hello
↓
World
```

---

# WarpUI.text

```text
WarpUI.text ID (内容)
```

指定したIDのtextを変更します。

対応：

```text
TextView
Button
CheckBox
RadioButton
Switch
ToggleButton
EditText
その他textを持つView
```

---

# WarpUI.textColor

```text
WarpUI.textColor ID (#ffffff)
```

指定Viewの文字色を変更します。

XML属性でいう：

```xml
baram:textColor
```

と同じです。

---

# WarpUI.background

```text
WarpUI.background ID (#000000)
```

XML：

```xml
baram:background
```

と同じです。

---

# WarpUI.textSize

```text
WarpUI.textSize ID (20sp)
```

XML：

```xml
baram:textSize
```

と同じです。

---

# その他属性

原則としてXMLで利用可能な属性はWarpUIから変更可能です。

概念：

```text
WarpUI.属性名 ID (値)
```

例：

```text
WarpUI.layout_width title (200dp)
WarpUI.layout_height title (50dp)
WarpUI.alpha title (0.5)
WarpUI.padding title (10dp)
```

レイアウトに関係する値を変更した場合は再レイアウトを実行します。

---

# WarpUI.visibility

```text
WarpUI.visibility ID (visible)
WarpUI.visibility ID (invisible)
WarpUI.visibility ID (gone)
```

XMLの：

```xml
baram:visibility
```

と完全に同じ意味です。

---

# WarpUI.screen

```text
WarpUI.screen (ファイル名)
```

指定されたUI画面へ移動します。

・現在画面を破棄します
・指定ファイルを解析します
・新しいViewツリーを生成します
・新しい画面に対応したWarp 4 Scriptのイベントを有効にします

---

# WarpUI.OnClick

```text
WarpUI.OnClick ID {
    プログラム
}
```

指定Viewがクリックされた場合に実行されます。

例：

```text
WarpUI.OnClick button {
    WarpUI.text title (Clicked)
}
```

---

# OnClick対象

原則としてIDを持つすべてのViewに設定可能です。

特に：

```text
Button
ImageButton
TextView
CheckBox
RadioButton
Switch
ToggleButton
ImageView
LinearLayout
```

などでも使用可能です。

---

# WarpUI.editText

```text
WarpUI.editText ID (内容)
```

指定したEditText系Viewの内容を書き換えます。

例：

```text
WarpUI.editText username (Hello)
```

クリア：

```text
WarpUI.editText username ()
```

対応：

```text
EditText
AutoCompleteTextView
MultiAutoCompleteTextView
SearchView
```

---

# WarpUI.getText

```text
WarpUI.getText ID (変数名)
```

指定Viewの現在の文字列を既存変数へ格納します。

例：

```text
var.set username ()
WarpUI.getText input (username)
print(var[username])
```

・変数は事前に `var.set` されている必要があります
・存在しない変数の場合は代入しません
・取得不能なViewの場合は処理をスキップします

---

# Script展開

WarpUI命令にもWarp 4 Scriptの特殊展開規則を完全適用します。

例：

```text
var.set target (title)
var.set message (Hello)

WarpUI.text var[target] (var[message])
```

展開後：

```text
WarpUI.text title (Hello)
```

として処理します。

---

# 動的ID

```text
var.set id (button1)

WarpUI.text var[id] (Hello)
```

正常です。

---

# 動的値

```text
var.set color (#ff0000)

WarpUI.background title (var[color])
```

正常です。

---

# calcとの組み合わせ

```text
var.set size (10)

WarpUI.textSize title (calc[var[size] + 10]sp)
```

実装上、展開後：

```text
20sp
```

として解釈できる必要があります。

---

# OnClick内での展開

```text
var.set target (title)

WarpUI.OnClick button {
    WarpUI.text var[target] (Hello)
}
```

正常に動作します。

---

# IDが存在しない場合

```text
WarpUI.text unknown (Hello)
```

対象IDが存在しない場合：

・処理をスキップします
・Script全体を停止してはいけません

---

# 型が一致しない場合

例：

```text
WarpUI.editText button (Hello)
```

ButtonにeditText操作を行った場合：

・その命令のみスキップします
・可能ならデバッグログを出力します
・Script全体を停止してはいけません

---

# 画面変更後のID

```text
WarpUI.screen (settings.xml)
```

を実行した場合、以前の画面だけに存在していたIDは無効になります。

新しい画面のIDだけが有効です。

---

# UIとScriptの実行順

標準：

```text
UIファイル読み込み
↓
XML解析
↓
Viewツリー作成
↓
初期レイアウト
↓
Warp 4 Script読み込み
↓
OnClickなどのイベント登録
↓
ユーザー操作
```

---

# 完全な例

UI：

```xml
<?xml version="1.0" encoding="utf-8"?>

<LinearLayout
    xmlns:baram="http://schemas.baram.com/apk/res/baram"
    baram:id="@+id/main"
    baram:layout_width="match_parent"
    baram:layout_height="match_parent"
    baram:orientation="vertical"
    baram:padding="16dp">

    <TextView
        baram:id="@+id/title"
        baram:layout_width="wrap_content"
        baram:layout_height="wrap_content"
        baram:text="Hello"
        baram:textSize="24sp"
        baram:textStyle="bold"/>

    <EditText
        baram:id="@+id/input"
        baram:layout_width="match_parent"
        baram:layout_height="wrap_content"
        baram:layout_marginTop="12dp"
        baram:hint="Name"
        baram:singleLine="true"/>

    <Button
        baram:id="@+id/button"
        baram:layout_width="wrap_content"
        baram:layout_height="wrap_content"
        baram:layout_marginTop="12dp"
        baram:text="Apply"/>

</LinearLayout>
```

Script：

```text
var.set name ()

WarpUI.OnClick button {
    WarpUI.getText input (name)
    WarpUI.text title (Hello, var[name])
    WarpUI.editText input ()
}
```

ユーザーが：

```text
Taro
```

と入力してボタンを押した場合：

```text
title = Hello, Taro
input = 空
```

となります。

---

# 究極の動的例

```text
var.set target (title)
var.set operation (text)
var.set value (Hello)

WarpUI.var[operation] var[target] (var[value])
```

展開によって：

```text
WarpUI.text title (Hello)
```

となるため、正常に実行できます。

Warp 4 Scriptでは展開処理が命令解析より前に行われるため、WarpUIの命令名、ID、値、属性名のいずれにも変数・定数展開を使用できます。

---

# 基本原則

Warp 4 UIは、

```text
XML = 初期UI
Warp 4 Script = 動的処理
```

として分離します。

XMLには：

```text
View
レイアウト
初期文字列
初期サイズ
初期色
初期状態
ID
```

を記述します。

Warp 4 Scriptには：

```text
クリック
値変更
画面遷移
入力取得
条件分岐
計算
繰り返し
```

などの動作を記述します。

UI側はScriptを知らなくても単体で描画可能でなければなりません。

Script側はIDを通してUIへアクセスします。
