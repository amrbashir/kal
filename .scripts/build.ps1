# Build the project
pnpm -r build
cargo build --release

# Create the dist directory if it doesn't exist
New-Item -Force "./dist" -Type Directory > $null

# Copy the kal.exe to the dist directory
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { './target' }
Copy-Item -Force "$targetDir/release/kal.exe" "./dist/kal.exe"
