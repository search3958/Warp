# MosaicBoard 5 — MB5-NX Foundation

MB5の宣言的UIとPicol/Tcl風スクリプトを組み合わせた、Rust製ソフトウェア描画ランナーの開発基盤です。UIとロジックを分離し、標準テーマを優先するSwiftUI風の宣言的設計を目指しています。

## Quick start

RustツールチェーンとCコンパイラを用意し、macOSでプロジェクトルートから実行します。

```sh
sh ./runner/rust/run.sh
sh ./runner/rust/run.sh 50%
sh ./runner/rust/run.sh ./example/components
```

`50%` はCPUクロックの変更ではなく、描画頻度と旧式インラインスクリプトの実行予算・待機時間を調整するソフトウェアプロファイルです。`100%` は最大60FPS設定です。新しいC VMの手続きや `fetch` の実行時間をCPUクロックのように制限するものではありません。

## Project layout

```text
example/components/
  config.ini
  app/main.mb5       # UI tree
  models/main.yaml   # navigation / dropdown / tab data
  scripts/main.mb5   # procedures / events / fetch
  resources/         # assets
docs/
  MB5_SPEC.md        # language and runtime specification
  DEVELOPMENT.md    # development guide
runner/rust/         # no_std runner + embedded C99 script VM
tools/mb5/           # lint and validation tools
tools/vscode/        # MB5 syntax highlighting / snippets
```

## Verify

```sh
sh ./tools/mb5/check.sh
```

この検査はMB5構造チェック、C VM単体テスト、サンプルスクリプトの構文・コールバック検査を実行します。Cargoが利用可能なら `cargo check` も実行します。

## Read next

- [MB5 language and runtime specification](docs/MB5_SPEC.md)
- [Development guide](docs/DEVELOPMENT.md)
- [Runner README](runner/rust/README.md)
- [VS Code language support](tools/vscode/README.md)

## Important limits

- インタラクティブウィンドウは現在macOS対応です。
- `fetch` はシステムの `curl` を使う同期通信で、通信中はUIイベントループが停止します。
- プロジェクトのスクリプトは任意のHTTP(S)ホストへアクセスできるため、信頼できないプロジェクトは実行しないでください。
- この基盤は完成した本番用コンパイラではありません。未実装項目と上限は仕様書を参照してください。

## 起動時にクラッシュする場合

今回のランタイムでは、固定容量のProjectデータ（約2.9 MiB）をスタック上に作成してAppへ値渡ししていたため、macOSのメインスレッドスタックを圧迫する可能性がありました。Projectは静的領域に置き、Appから排他的に参照する方式へ変更しています。スクリプトVMの大きな一時領域もヒープへ移動しています。

通常起動で問題が残る場合は、AppKitを使わない描画テストで切り分けできます。

```sh
sh ./runner/rust/run.sh --ppm > /tmp/mb5-preview.ppm
```

PPM出力もクラッシュする場合はプロジェクト読み込み・VM・フォント・ソフトウェア描画側、PPMは成功してウィンドウ起動だけ失敗する場合はmacOSのAppKit/CoreGraphics側を調査してください。
