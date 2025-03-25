# Exit if kal.exe is not found in ./dist
if (-not (Test-Path -Path "./dist/kal.exe")) {
    Write-Host -ForegroundColor Red "Error: kal.exe not found in ./dist, run build.ps1 first"
    exit 1
}

# Copy the kal.exe to the installer directory
Copy-Item -Force "./dist/kal.exe" "./installer/kal.exe"

# Copy the icon.ico to the installer directory
Copy-Item -Force "./kal/assets/icon.ico" "./installer/icon.ico"

# Create the installer
makensis /V4 "./installer/installer.nsi"

# Move the installer to the dist directory
Move-Item -Force "./installer/kal-setup.exe" "./dist/kal-setup.exe"

# Compress the kal.exe to kal.zip
Compress-Archive -Update "./dist/kal.exe" "./dist/kal.zip"

# Remove artifacts
Remove-Item -Force "./installer/kal.exe"
Remove-Item -Force "./installer/icon.ico"
