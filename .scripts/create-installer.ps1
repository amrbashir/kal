Copy-Item -Force "kal/assets/icon.ico" "installer/icon.ico"

if ($env:CARGO_TARGET_DIR) {
  Copy-Item -Force "$env:CARGO_TARGET_DIR/release/kal.exe" "installer/kal.exe"
} else {
  Copy-Item -Force "./target/release/kal.exe" "installer/kal.exe"
}

makensis /V4 installer/installer.nsi

New-Item -Force "dist" -Type Directory > $null

Move-Item -Force "installer/kal.exe" "dist/kal.exe"
Move-Item -Force "installer/kal-setup.exe" "dist/kal-setup.exe"
