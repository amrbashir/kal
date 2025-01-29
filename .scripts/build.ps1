pnpm -r build
cargo build --release

if ($env:CARGO_TARGET_DIR) {
  Copy-Item -Force "$env:CARGO_TARGET_DIR/release/kal.exe" "installer/kal.exe"
} else {
  Copy-Item -Force "./target/release/kal.exe" "installer/kal.exe"
}

makensis /V4 installer/installer.nsi

Move-Item -Force "installer/kal.exe" "kal.exe"
Move-Item -Force "installer/kal-setup.exe" "kal-setup.exe"
