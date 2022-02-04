# "pnpm dev" (vite dev server) process info
$pnpmDevInfo = New-Object System.Diagnostics.ProcessStartInfo "pnpm"
$pnpmDevInfo.Arguments = "dev"
$pnpmDevInfo.WorkingDirectory = "$PSScriptRoot/../ui"
$pnpmDev = New-Object System.Diagnostics.Process
$pnpmDev.StartInfo = $pnpmDevInfo

# "cargo run" process info
$cargoRunInfo = New-Object System.Diagnostics.ProcessStartInfo "cargo"
$cargoRunInfo.Arguments = "run"
$cargoRunInfo.WorkingDirectory = "$PSScriptRoot/../core"
$cargoRun = New-Object System.Diagnostics.Process
$cargoRun.StartInfo = $cargoRunInfo

# file watcher for the rust code
$fileWatcher = New-Object System.IO.FileSystemWatcher
$fileWatcher.Path = "$PSScriptRoot/../core/src"
$fileWatcher.Filter = "*"
$fileWatcher.IncludeSubdirectories = $true
function RestartCargoRun {
  $cargoRun.kill()
  [void]$cargoRun.start()
}

$watching = $true

[void]$pnpmDev.start()
[void]$cargoRun.start()

do {
  $result = $fileWatcher.WaitForChanged([System.IO.WatcherChangeTypes]::Changed, 1000)
  if ($result.TimedOut) { continue }
  RestartCargoRun
}
while($watching)
