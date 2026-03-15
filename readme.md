# Warp

AndroidとWebを一つのコードで動かす、とにかく簡単に扱えるのUI記述言語。将来的にiOSにも対応予定です。
ちょっとしたアプリならわざわざ複雑なものを使わない方が楽でしょう？

---

## コンポーネントの使用方法

各コンポーネントの基本機能とサンプルです。

### 1. Header (ヘッダー)
画面上部に固定されるタイトルバーです。右側にボタンなどのコンポーネントを配置できます。
```warp
Header(
    text: "タイトル"
    button(text: "保存")
)
```

### 2. card (カード)
コンテンツをグループ化するコンテナです。自動的にMaterial 3スタイルの背景と影が適用されます。
```warp
card(
    text: "見出し"
    text(text: "これはカード内のテキストです。")
)
```

### 3. button / tonalButton (ボタン)
アクションを実行するためのボタンです。`button`は強調されたスタイル、`tonalButton`は控えめな背景色のスタイルになります。
```warp
// 標準ボタン
button(
    text: "送信"
    oneClick: --status = "完了"
)

// 薄い色のボタン
tonalButton(
    text: "キャンセル"
    color: gray
)
```

### 4. text (テキスト)
文字列を表示します。変数を参照して動的に内容を書き換えることができます。
```warp
text(
    text: "ステータス: " + --status
    color: blue
)
```

### 5. Layout (スタック)
要素を並べるためのコンテナです。`hStack`は横並び、`vStack`は縦並びに配置します。
```warp
hStack(
    button(text: "左側")
    button(text: "右側")
)

vStack(
    id: dynamicList
    // idを指定することで後から要素を追加できます
)
```

### 6. screen (画面)
アプリケーションの各画面を定義します。`id`で識別し、画面遷移の対象となります。
```warp
screen(
    id: main
    Header(text: "ホーム")
    button(text: "設定へ", oneClick: setScreen(settings))
)
```

---

## スクリプト

### 条件分岐 (@script)
複数の処理や条件分岐をまとめて定義できます。`script(名前)`で呼び出します。
```warp
@check(
    if: --val = 1 ( 
        --resultText = "正解",
        show(successIcon)
    )
    elseIf: --val = 0 ( 
        --resultText = "不正解" 
    )
)
```

### 利用可能なアクション
- `setScreen(id)` : 指定したIDの画面へ切り替え
- `add(id: 'node')` : 指定したコンテナに動的に要素を追加
- `del(id: nodeName)` : 要素を削除
- `clr(id)` : コンテナ内の動的要素をすべて削除
- `show(id)` / `hide(id)` : 指定した要素の表示・非表示
- `calc(式)` : 四則演算を実行（変数が数値の場合）
- `reset(now)` : アプリの状態を初期化

---

## 状況
- **Web**: 最も高速に対応します。
- **Android**: ほとんど対応しています
- **BaramOS ネイティブUI**: 基礎部分のみ対応しています。
- **iOS**: 開発予定
