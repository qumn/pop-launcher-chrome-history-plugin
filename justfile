build:
    cargo build --release
build_sway:
    cargo build --release --features sway

_install:
    mkdir -p ~/.local/share/pop-launcher/plugins/chrome
    install -Dm0755 ./target/release/pop-launcher-chrome-history-plugin ~/.local/share/pop-launcher/plugins/chrome/ch
    install -Dm644 plugin.ron ~/.local/share/pop-launcher/plugins/chrome/plugin.ron

install:
    just build
    just _install

install-sway:
    just build_sway
    just _install
