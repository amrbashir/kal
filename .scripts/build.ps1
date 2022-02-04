$currentDir = Get-Location
Set-Location $PSScriptRoot/../ui
pnpm build
Set-Location $PSScriptRoot/../core
cargo build --release
Set-Location $currentDir
