# Mosaic Board 5

Mosaic Board 5 は、ボード単位でUIを構築し、独自DSLによって画面構成や処理を記述する言語・実行環境です。

この文書では、現在確定している仕様を記載します。まだ定義されていない機能や挙動については、独自の解釈で補完せず「未規定」としています。

## 1. プロジェクト構成

```text
project/
├── app/
│   └── main.mb5
├── models/
│   └── main.yaml
├── resources/
│   ├── icon.svg
│   └── ...
├── config.ini
└── README.md
```

| ファイル・ディレクトリ | 役割 |
|---|---|
| `app/*.mb5` | UI構成、ボード、struct、スクリプトなど |
| `models/*.yaml` | UIで使用するモデル・設定・テキスト・イベントなど |
| `resources/` | アイコンなどのアセット |
| `config.ini` | プロジェクト設定 |
| `README.md` | 仕様・プロジェクト説明 |

### アセットの参照

アセットの相対パスは、常にプロジェクト内の `resources/` を基準に解決します。

例えば、次の指定は `resources/test/happyegg.svg` を参照します。

```yaml
icon: test/happyegg.svg
```

アセットは `resources/` の外に配置しないことを前提とします。

## 2. MB5の基本構造

### メインボード

`@mainBoard` はメインボードの定義を示します。

```text
@mainBoard

board home{
    text(about-list)
}
```

`board` は画面・表示領域の単位です。ボード間の移動には `moveboard()` を使用します。

```text
button(
    config:text: "別画面へ移動する",
    event: moveboard(board1)
)
```

### メインスクリプト

`@mainScript` はメインスクリプトの定義を示します。

```text
@mainScript

script hello{
    wait(1000)
    closewelcome()
}
```

UIの定義とスクリプトの定義は明確に分離します。UI記述とスクリプト記述を同一のものとして扱いません。

## 3. コンポーネント

コンポーネントは、`alert()`、`text()`、`list()`、`button()` などの命令で配置します。

```text
board home{
    alert(welcome-text)
    .id("welcome-alert")
    .display(show)

    button(config:text:"Primary")
    .type(primary)
}
```

コンポーネントに対する追加設定は、Swiftのようなチェーン形式で記述します。

```text
button(button-text)
.type(primary)
.bgColor(#cc0000)
.fgColor(#eeddcc)
.size(large)
.radius(24)
.width(50px)
.display(show)
```

各修飾子は直前の対象に対する設定として記述します。修飾子の具体的な引数・型・対応範囲は、各命令の仕様に従います。

### コンポーネントの追加・削除

`@struct` で定義したコンポーネントは `addComponent()` で追加できます。

同じstructを複数回追加することができます。これはコンポーネント定義の再利用を想定した仕様です。

追加したコンポーネントと、最初から配置されていたコンポーネントは区別しません。同名のコンポーネントを追加した場合も、通常どおり追加されます。

`deleteComponent()` は、対象ボード内にある該当コンポーネントをすべて削除します。

```text
deleteComponent(luckyalert)
.target(home)
```

`.target()` を指定しない場合は、実行時点で現在のボードが対象です。

## 4. struct

`@struct` は、再利用するコンポーネント構成を定義します。

```text
@struct

components luckyalert{
    alert(luckyalert-text)
}
```

定義したstructは、次のように追加できます。

```text
addComponent(luckyalert)
```

同じstructを複数回追加でき、追加された各要素を個別のインスタンスとして識別・管理する仕様ではありません。

## 5. スクリプトとmessage

スクリプトはUIとは独立した処理として定義します。

```text
script luckyalert{
    message(show){
        addComponent(luckyalert)
        .target(home)

        wait(5000)

        deleteComponent(luckyalert)
    }

    message(close){
        deleteComponent(luckyalert)
        .target(home)
    }
}
```

`message()` は処理の分岐を表します。`case` / `switch` に近い役割を持ち、括弧内の値は分岐に渡される引数として扱います。

YAMLやDSLから、次のように呼び出します。

```text
luckyalert(show)
```

### 非同期処理とボードの対象

`wait()` などの待機を含む処理では、`.target()` を指定しない限り、処理が実行される時点の現在のボードを基準にします。

待機中にユーザーが別のボードへ移動した場合、後続処理もその時点の現在のボードを対象とします。

特定のボードを対象にしたい場合は、`.target(boardName)` を明示的に指定することを推奨します。

## 6. YAMLモデル

YAMLは、UIで使用するテキスト・設定・モデルデータ・イベントなどを定義します。

```yaml
welcome-text:
  config:
    target: alert
    title: "Hello, Developer!"
    message: "Mosaic Board 5 へようこそ"
  events:
    close: closewelcome()
  model:
    close: "Close now"
```

モデルは名前をキーとして定義し、DSLから参照します。

```text
alert(welcome-text)
```

### config / model / event(s)

| キー | 役割 |
|---|---|
| `config` | 対象コンポーネントの設定 |
| `model` | コンポーネント内で参照するデータ |
| `event` | 単一のイベント |
| `events` | 複数のイベント |

`event` と `events` は明確に区別します。単一イベントには `event`、複数イベントの定義には `events` を使用します。

### model内の識別子

例えば、次の定義では `list` と `button` は `model` 内のデータを識別するためのIDです。

```yaml
components:
  config:
    title: "要素一覧"
    target: list
  model:
    list: "リスト"
    button: "ボタン"
```

`list` や `button` という名前だけでUIコンポーネントそのものを意味するわけではありません。所属する `model` の中でデータを識別するキーです。

## 7. インラインYAML

DSL内には、YAML形式に近いデータを直接記述できます。

### `[]` で囲まない場合

`[]` で囲まない記述は、1行につき1つの指定として扱います。

```text
list(
    config:title: "タイトル"
    config:text: "本文を追加できます"
)
```

この形式では、複数行にまたがる値や複数行のオブジェクトを1つの指定として記述できません。

### `[]` で囲む場合

複数行のデータをインラインYAML形式で記述する場合は、角括弧 `[]` で囲みます。

```text
list(
    [
        model:
            1: "項目1"
            2: "項目2"
            3: "項目3"
    ]
)
```

### コンパイル時のエラー

次のような、値や内容を持たない定義はコンパイルエラーです。

```text
model:
```

また、`[]` で囲まれていない状態で複数行のデータを記述した場合も、1行のみという文法に反するためコンパイルエラーです。

## 8. 設定の競合

YAML側の設定とDSL側の設定が競合する場合、どちらかを優先して上書きすることはしません。競合エラーとして扱います。

設定を両方に重複して定義しないようにしてください。

## 9. コンパイル時の検証

少なくとも、次のような不正な記述はコンパイル段階でエラーとして扱います。

- 内容のない `model:` などの定義
- `[]` で囲まれていない複数行のインラインデータ
- YAMLとDSLの設定の競合

未定義のモデル参照など、その他の検証項目・エラー形式の詳細は、今後の仕様で定義します。

## 10. プロジェクト設定

`config.ini` ではプロジェクト全体の設定を定義します。

```ini
version=5
name=Mosaic Board 5 Tester
icon=icon.svg
```

`icon` などのアセットパスは `resources/` を基準に解決します。

## 11. 今後定義する項目

以下は現時点で仕様が確定していないため、実装時に明確な定義が必要です。

- 各命令・修飾子が受け付ける引数と型
- 文字列、数値、真偽値、配列、オブジェクトの型規則
- 未定義のモデル・イベント・structを参照した場合のエラー
- 同時に複数のイベントが発生した場合の実行順序
- スクリプトの戻り値・終了条件
- `wait()` 中の処理中断やエラー発生時の扱い
- コンポーネントの表示・非表示・削除に関するライフサイクル

これらは現時点で機能が不足しているという意味ではなく、今後の拡張に向けて仕様を確定させる項目です。
