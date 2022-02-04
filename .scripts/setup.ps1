$currentDir = Get-Location
git config core.hooksPath .git-hooks
Set-Location $PSScriptRoot/../ui
pnpm i
Set-Location $PSScriptRoot/../core
cargo update
Set-Location $currentDir
