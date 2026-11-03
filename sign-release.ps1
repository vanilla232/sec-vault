$GpgId = "F494311C4FA562AF" 
$BundlePath = "src-tauri/target/release/bundle"

Write-Host "Генерация контрольных сумм (SHA-256)..."
Get-ChildItem -Path $BundlePath -Recurse -Include *.msi, *.exe | Get-FileHash -Algorithm SHA256 | Select-Object Hash, Path | Out-File -FilePath "checksums.txt" -Encoding utf8

Write-Host "Подпись файла checksums.txt..."
gpg --default-key $GpgId --detach-sign --armor checksums.txt

Write-Host "Готово! Файлы checksums.txt и checksums.txt.asc созданы."