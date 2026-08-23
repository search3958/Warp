# Warp 4 example compiler

`src` にあるWarp 4 UIとScriptを、外部ファイル不要の単一HTMLへ変換します。

```sh
./example/run.sh
```

デフォルトでは次の場所を使います。

- 入力: `example/src`
- 出力: `example/output/index.html`

入出力先は引数で変更できます。

```sh
./example/run.sh ./path/to/src ./path/to/output
```

`config.ini` の `screen` を初期画面として使い、入力ディレクトリ内の `.w3u/.w4u` と `.w3s/.w4s` を同じ画面名で関連付けます。生成HTMLにはUIの初期状態、`WarpUI.OnClick`、画面遷移、変数展開、`calc[...]`、`if` を実行するランタイムがインラインで含まれます。
