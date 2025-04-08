$version = $args[0];

foreach ($cargoToml in Get-ChildItem "kal*/Cargo.toml") {
  $path = $cargoToml.FullName
  (Get-Content $path) -replace "version = `"[0-9].[0-9].[0-9]`"", "version = `"$version`"" | Set-Content $path

  $parentDir = $cargoToml.DirectoryName
  $parentDir = $parentDir.Substring($parentDir.LastIndexOf("\") + 1)
  cargo update --package $parentDir
}

$path = "kal-ui/package.json"
(Get-Content $path) -replace "`"version`": `"[0-9].[0-9].[0-9]`"", "`"version`": `"$version`"" | Set-Content $path

$path = "CHANGELOG.md"
$date = Get-Date -Format "yyyy-MM-dd"
(Get-Content $path) -replace "## \[Unreleased\]", "## [Unreleased]`n`n## [$version] - $date" | Set-Content $path

git add .
git commit -m "release: v$version";
git tag "v$version"
git push
git push --tags
