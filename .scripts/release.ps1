$path = "kal/Cargo.toml"
(Get-Content $path).replace("version = `"0.0.0`"", "version = `"$args`"") | Set-Content $path

$path = "kal-ui/package.json"
(Get-Content $path).replace("`"version`": `"0.0.0`"", "`"version`": `"$args`"") | Set-Content $path

git add .
git commit -m "release: v$args";
git tag "v$args"
git push
git push --tags
