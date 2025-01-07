# "deno task dev" (vite dev server) process info
$devTaskInfo = New-Object System.Diagnostics.ProcessStartInfo "powershell"
$devTaskInfo.Arguments = "-Command deno task --recursive dev"
$devTaskInfo.WorkingDirectory = Split-Path -Parent $PSScriptRoot
$denoTask = New-Object System.Diagnostics.Process
$denoTask.StartInfo = $devTaskInfo

# "cargo run" process info
$cargoRunInfo = New-Object System.Diagnostics.ProcessStartInfo "cargo"
$cargoRunInfo.Arguments = "run"
$cargoRunInfo.WorkingDirectory = Split-Path -Parent $PSScriptRoot
$cargoRun = New-Object System.Diagnostics.Process
$cargoRun.StartInfo = $cargoRunInfo

# Start
[void]$denoTask.start()
[void]$cargoRun.start()

# Setup file watcher for the rust code
$fileWatcher = New-Object System.IO.FileSystemWatcher
$fileWatcher.Path = Split-Path -Parent $PSScriptRoot
$fileWatcher.Filter = "*"
$fileWatcher.IncludeSubdirectories = $true

try {
  do {
    $result = $fileWatcher.WaitForChanged([System.IO.WatcherChangeTypes]::Changed, 1000)
    if (
      ($result.Name -like 'src\core\*') -or
      ($result.Name -like 'src\common\*.rs') -or
      ($result.Name -like 'Cargo.toml') -or
      ($result.Name -like 'Cargo.lock')
    ) {
      $cargoRun.kill()
      [void]$cargoRun.start()
    }
  }
  while($true)
} finally {
  $fileWatcher.Dispose();
  $cargoRun.kill();
  $denoTask.kill();
}
