$path = "CHANGELOG.md"
$out = if ($args[0]) { $args[0] } else { "RELEASE_NOTES.md" }

$changelog = Get-Content $path

# Extract latest release notes
$releaseContent = @()
$isLatestRelease = $false
$isInLatestRelease = $false

foreach ($line in $changelog) {
    # Skip [Unreleased] section
    if ($line -match "^##\s+\[Unreleased\]") {
        continue
    }
    # Check for release header (e.g. ## [1.0.0] - 2023-01-01)
    elseif ($line -match "^##\s+\[") {
        # First actual release header found
        if (!$isLatestRelease) {
            $isLatestRelease = $true
            $isInLatestRelease = $true
        } else {
            # Next release header found, stop capturing
            $isInLatestRelease = $false
            break
        }
    } elseif ($isInLatestRelease) {
        $releaseContent += $line
    }
}

# Write to output file
$releaseContent | Out-File -FilePath $out -Encoding utf8
