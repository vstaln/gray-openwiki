#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/gray-openwiki.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/gray-openwiki" ;;
    *) rel="x86_64-apple-darwin/gray-openwiki" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/gray-openwiki" ;;
    *) rel="x86_64-unknown-linux-musl/gray-openwiki" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/gray-openwiki.exe" "$here/target/release/gray-openwiki" "$here/target/debug/gray-openwiki.exe" "$here/target/debug/gray-openwiki"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "gray-openwiki binary not found in $here" >&2
exit 1
