$path = "kal/Cargo.toml"
(Get-Content $path) -replace "version = `"[0-9].[0-9].[0-9]`"", "version = `"$args`"" | Set-Content $path

$path = "kal-ui/package.json"
(Get-Content $path) -replace "`"version`": `"[0-9].[0-9].[0-9]`"", "`"version`": `"$args`"" | Set-Content $path

Start-Sleep -Seconds 2

git add .
git commit -m "release: v$args";
git tag "v$args"
git push
git push --tags
