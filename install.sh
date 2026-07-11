#!/usr/bin/env sh
set -eu

REPO="${UNBIND_REPO:-felipersas/unbind}"
VERSION="${UNBIND_VERSION:-latest}"
INSTALL_DIR="${UNBIND_INSTALL_DIR:-$HOME/.local/bin}"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf "error: required command not found: %s\n" "$1" >&2
    exit 1
  }
}

target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os:$arch" in
    Linux:x86_64) printf "x86_64-unknown-linux-gnu" ;;
    Darwin:arm64) printf "aarch64-apple-darwin" ;;
    Darwin:x86_64) printf "x86_64-apple-darwin" ;;
    *)
      printf "error: unsupported platform: %s %s\n" "$os" "$arch" >&2
      exit 1
      ;;
  esac
}

latest_version() {
  url="$(curl -fsSLI -o /dev/null -w "%{url_effective}" "https://github.com/$REPO/releases/latest")"
  version="${url##*/}"
  [ -n "$version" ] || {
    printf "error: could not resolve latest release for %s\n" "$REPO" >&2
    exit 1
  }
  printf "%s" "$version"
}

verify_checksum() {
  file="$1"
  checksum_file="$2"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$checksum_file"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "$checksum_file"
  else
    printf "warning: sha256sum/shasum not found; skipping checksum verification\n" >&2
    return 0
  fi

  [ -f "$file" ]
}

need curl
need install
need tar

[ "$VERSION" = "latest" ] && VERSION="$(latest_version)"
TARGET="$(target)"
ASSET="unbind-$VERSION-$TARGET.tar.gz"
BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

printf "Installing unbind %s for %s...\n" "$VERSION" "$TARGET"

curl -fsSL "$BASE_URL/$ASSET" -o "$TMP_DIR/$ASSET"
if curl -fsSL "$BASE_URL/$ASSET.sha256" -o "$TMP_DIR/$ASSET.sha256"; then
  (cd "$TMP_DIR" && verify_checksum "$ASSET" "$ASSET.sha256")
else
  printf "warning: checksum file not found; continuing without verification\n" >&2
fi

mkdir -p "$INSTALL_DIR"
tar -xzf "$TMP_DIR/$ASSET" -C "$TMP_DIR"
install -m 755 "$TMP_DIR/unbind" "$INSTALL_DIR/unbind"

printf "Installed unbind to %s/unbind\n" "$INSTALL_DIR"
if ! command -v unbind >/dev/null 2>&1; then
  printf "Add this to your shell config if needed:\n"
  printf "  export PATH=\"%s:\$PATH\"\n" "$INSTALL_DIR"
fi
