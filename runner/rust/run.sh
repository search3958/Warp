#!/bin/sh
# Mosaic Board 5 ランナーの起動スクリプト
#
# 使い方（プロジェクトのルートで実行）:
#   sh ./runner/rust/run.sh                    # ./example/components を開く
#   sh ./runner/rust/run.sh path/to/components # 指定した components を開く
#   sh ./runner/rust/run.sh components --ppm > out.ppm  # ウィンドウを開かず PPM を出力
#
# 最初の引数が -- で始まらない場合は components のパスとして扱い、
# それ以降の引数はそのままランナーに渡す。

set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
PROJECT_ROOT=$(cd "$HERE/../.." && pwd)

# 既定: ルート直下の example/components、なければ components
if [ -d "$PROJECT_ROOT/example/components" ]; then
    COMPONENTS="$PROJECT_ROOT/example/components"
else
    COMPONENTS="$PROJECT_ROOT/components"
fi
if [ $# -gt 0 ]; then
    case "$1" in
        --*|-h)
            ;;
        *)
            COMPONENTS="$1"
            shift
            ;;
    esac
fi

# 相対パスは呼び出し時のカレントディレクトリ基準にする
case "$COMPONENTS" in
    /*) ;;
    *) COMPONENTS="$(pwd)/$COMPONENTS" ;;
esac

if [ ! -f "$COMPONENTS/app/main.mb5" ]; then
    echo "components が見つかりません: $COMPONENTS" >&2
    echo "（app/main.mb5 が存在するディレクトリを指定してください）" >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo が見つかりません。https://rustup.rs からRustをインストールしてください。" >&2
    exit 1
fi

cargo build --release --quiet --manifest-path "$HERE/Cargo.toml"

exec "$HERE/target/release/mb5-runner" "$COMPONENTS" "$@"
