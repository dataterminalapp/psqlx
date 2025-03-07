#!/bin/bash

ARCH="macos-$(uname -m)"
REPO="dataterminalapp/psqlx"
TAP_REPO="dataterminalapp/homebrew-tap"
VERSION="0.1.0"
BIN_NAME="psqlx"
PSQL_BUILD_DIR="/usr/local/pgsql/bin"
BUILD_DIR="./build"
PLUGIN_DIR="$BUILD_DIR/plugins"
ARCHIVE_NAME="${BIN_NAME}-${ARCH}.tar.gz"
FORMULA_FILE="Formula/${BIN_NAME}.rb"

echo "🔨 Building ${BIN_NAME}..."
./bin/build.sh

echo "🫰 Grouping build..."
mkdir -p "$BUILD_DIR"
cp "$PSQL_BUILD_DIR/${BIN_NAME}" "$BUILD_DIR"

echo "📦 Packaging binary..."
tar -czvf "$ARCHIVE_NAME" -C "$BUILD_DIR" .

echo "📝 Calculating SHA256..."
SHA256=$(shasum -a 256 "$ARCHIVE_NAME" | awk '{print $1}')

echo "🚀 Uploading to GitHub Releases..."
gh release create "v$VERSION" "$ARCHIVE_NAME" --repo "$REPO" --notes "Release $VERSION"

echo "🌿 Cloning Homebrew tap..."
git clone "https://github.com/$TAP_REPO.git" homebrew-tap
cd homebrew-tap

echo "✏️ Updating Formula..."
cat <<EOF > "$FORMULA_FILE"
class Psqlx < Formula
  desc "Psql-fork focused on extensibility"
  homepage "https://github.com/$REPO"
  url "https://github.com/$REPO/releases/download/v$VERSION/$ARCHIVE_NAME"
  sha256 "$SHA256"
  version "$VERSION"

  depends_on "libpq"
  depends_on "libedit"

  def install
    bin.install "$BIN_NAME"
  end

  test do
    system "\#{bin}/$BIN_NAME", "--version"
  end
end
EOF

echo "📤 Pushing Formula update..."
git add "$FORMULA_FILE"
git commit -m "Update $BIN_NAME to v$VERSION"
git push origin main

echo "✅ Done! Users can now run:"
echo "    brew update && brew install dataterminalapp/tap/$BIN_NAME"
