Copy-Item -Force "kal/assets/icon.ico" "installer/icon.ico"

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { './target' }

$exe = "$targetDir/release/kal.exe"

if (!(Test-Path $exe)) {
  & "./.scripts/build.ps1"
}

Copy-Item -Force $exe "installer/kal.exe"

makensis /V4 "installer/installer.nsi"

New-Item -Force "dist" -Type Directory > $null

Move-Item -Force "installer/kal.exe" "dist/kal.exe"
Move-Item -Force "installer/kal-setup.exe" "dist/kal-setup.exe"

Compress-Archive "dist/kal.exe" "dist/kal.zip"
