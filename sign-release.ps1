$GpgId = "F494311C4FA562AF" 
$BundlePath = "src-tauri/target/release/bundle"

Write-Host 
Get-ChildItem -Path $BundlePath -Recurse -Include *.msi, *.exe | Get-FileHash -Algorithm SHA256 | Select-Object Hash, Path | Out-File -FilePath "checksums.txt" -Encoding utf8

Write-Host 
gpg --default-key $GpgId --detach-sign --armor checksums.txt

Write-Host 