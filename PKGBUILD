pkgname=velocitext
pkgver=0.0.1
pkgrel=1
pkgdesc="Velocitext local-first notes app"
arch=("x86_64")
url="https://example.com/velocitext"
license=("MIT")
depends=("webkit2gtk" "gtk3" "libsoup" "libappindicator-gtk3" "libayatana-appindicator")
makedepends=("cargo" "nodejs" "npm" "pkgconf")
source=("$pkgname-$pkgver.tar.gz")
sha256sums=("SKIP")

build() {
  cd "$srcdir/$pkgname-$pkgver"
  npm install
  npm run build
  cargo build --release -p velocitext
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 "src-tauri/target/release/velocitext" "$pkgdir/usr/bin/velocitext"
  install -Dm644 "src-tauri/icons/icon.png" "$pkgdir/usr/share/icons/hicolor/512x512/apps/velocitext.png"
  install -Dm644 "src-tauri/velocitext.desktop" "$pkgdir/usr/share/applications/velocitext.desktop"
}
