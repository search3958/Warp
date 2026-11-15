#!/bin/sh
# Mosaic Board 5 ランナーの起動スクリプト
#
# 使い方（プロジェクトのルートで実行）:
#   sh ./runner/rust/run.sh                    # ./example/components を開く
#   sh ./runner/rust/run.sh path/to/components # 指定した components を開く
#   sh ./runner/rust/run.sh 200MHz              # 200MHz 相当の速度上限
#   sh ./runner/rust/run.sh 200MHz path/to/components
#   sh ./runner/rust/run.sh components --ppm > out.ppm  # ウィンドウを開かず PPM を出力
#
# 速度指定は先頭に置く。components パスと --ppm はその後に指定する。

set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
PROJECT_ROOT=$(cd "$HERE/../.." && pwd)

# 既定: ルート直下の example/components、なければ components
if [ -d "$PROJECT_ROOT/example/components" ]; then
    COMPONENTS="$PROJECT_ROOT/example/components"
else
    COMPONENTS="$PROJECT_ROOT/components"
fi
SPEED_ARGS=""
if [ $# -gt 0 ]; then
    case "$1" in
        *MHz)
            SPEED_VALUE=${1%MHz}
            case "$SPEED_VALUE" in
                ''|*[!0-9]*) echo "無効な速度指定です: $1 (例: 200MHz)" >&2; exit 2 ;;
            esac
            if [ "$SPEED_VALUE" -lt 1 ] || [ "$SPEED_VALUE" -gt 100000 ]; then
                echo "速度は 1MHz〜100000MHz の範囲で指定してください: $1" >&2
                exit 2
            fi
            SPEED_ARGS=$SPEED_VALUE
            shift
            ;;
    esac
fi
if [ $# -gt 0 ]; then
    case "$1" in
        --*|-h) ;;
        *) COMPONENTS="$1"; shift ;;
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


if [ -n "$SPEED_ARGS" ]; then
    exec "$HERE/target/release/mb5-runner" "$COMPONENTS" --speed-mhz "$SPEED_ARGS" "$@"
else
    exec "$HERE/target/release/mb5-runner" "$COMPONENTS" "$@"
fi
