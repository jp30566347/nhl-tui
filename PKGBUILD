# Maintainer: JP Melanson <1215807+jp30566347@users.noreply.github.com>
pkgname=nhl-tui
pkgver=0.1.1
pkgrel=1
pkgdesc="NHL scores, standings and stats in your terminal"
arch=('x86_64' 'aarch64')
url="https://github.com/jp30566347/nhl-tui"
license=('MIT')
# makepkg's own LTO adds -flto to CFLAGS, which makes the cc crate emit
# bitcode for ring's C sources while its hand-written assembly stays as
# objects; linking then fails on undefined ring_core_* symbols. Cargo does
# its own link-time optimisation through the release profile anyway.
options=('!lto')
depends=('gcc-libs')
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release
}

check() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    # The network-backed checks are ignored by default, so this stays offline.
    cargo test --frozen --release
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm0755 -t "$pkgdir/usr/bin/" "target/release/$pkgname"
    install -Dm0644 -t "$pkgdir/usr/share/licenses/$pkgname/" LICENSE
    install -Dm0644 -t "$pkgdir/usr/share/doc/$pkgname/" README.md
}
