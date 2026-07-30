# Warp 3 Compiler

`.w2u` の UI と `.w2s` の動作を、ブラウザでそのまま開ける HTML・JavaScript・CSS に変換する Rust 製コンパイラです。

```sh
./w3cp.sh
open dist/index.html
```

出力先を変える場合は `./w3cp.sh path/to/output` とします。

## ファイル

- `config.ini`: `version=3`、起動画面 `screen`、アプリ名 `name`
- `<画面名>.w2u`: UI。ファイル名が画面名になります
- `.w2s`: `config { script("...") }` から読み込むイベントと関数

### UI の例

```text
config {
    title("画面タイトル")
    script("app.w2s")
}

head,scroll-point.top {
    text("見出し")
}

button.save {
    text("保存")
    type("primary")
}
```

### スクリプトの例

```ini
[onClick = save]
count = +1
setText display = count
print = "saved"

[fun = reset]
count = "0"
wait = 100ms
fun = update
```

対応命令は `scroll`、`screen`、`print`、`wait`、`fun`、`setText`、`getText` と変数への代入・加減算です。
