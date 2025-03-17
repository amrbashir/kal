$path = "CHANGELOG.md"
$out = if ($args[0]) { $args[0] } else { "RELEASE_NOTES.md" }


$changelog = Get-Content $path

$matchCount = 0
$index = -1
for ($i = 0; $i -lt $changelog.Length; $i++) {
    if ($changelog[$i] -match '^(?!###)##') {
        $matchCount++
        if ($matchCount -eq 2) {
            $index = $i - 1
            break
        }
    }
}

$changelog[1..$index] | Set-Content $out
