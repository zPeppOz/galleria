# Spike 1, criteri 1.6 e 1.7: costruisce nella Sandbox i due MSI (0.0.1 e 0.0.2) dell'app Tauri con il servizio dentro.
# Serve: toolchain e build calda (windows-lab/sandbox/install-toolchain.ps1, app-warm.ps1), binari in C:\sbx\in\msi.
# La chiave dell'updater e' USA E GETTA: nasce qui dentro, non esce dalla Sandbox, NON e' la chiave di release.
$ErrorActionPreference = 'Continue'
$env:RUSTUP_HOME = 'C:\tools\rustup'; $env:CARGO_HOME = 'C:\tools\cargo'
$env:Path += ';C:\tools\cargo\bin;C:\tools\w64devkit\bin'
$OUT = 'C:\sbx\out\msi'; $K = 'C:\gl\keys'; $REL = 'C:\gl\rel'
New-Item -ItemType Directory -Force $OUT, $K, $REL, C:\gl\stage | Out-Null
while (-not (Test-Path C:\sbx\out\app-warm.done)) { Start-Sleep 3 }
"warm build: " + (Get-Content C:\sbx\out\app-warm.log | Select-Object -Last 1)
Copy-Item C:\sbx\in\src\app\src-tauri\src\main.rs C:\gl\app\src-tauri\src\main.rs -Force
Copy-Item C:\sbx\in\src\app\src-tauri\wix\service.wxs C:\gl\app\src-tauri\wix\service.wxs -Force
Set-Location C:\gl\app\src-tauri
cargo tauri -V

# Nella Sandbox manca il motore VBScript: la validazione ICE di light.exe fallisce (errore 2738, LGHT0217).
# Il primo `cargo tauri build` scarica WiX; se light non e' gia' sostituito, lo si sostituisce con uno che aggiunge -sval.
function Patch-Light {
  $wix = "$env:LOCALAPPDATA\tauri\WixTools314"
  if ((Test-Path "$wix\light.exe") -and -not (Test-Path "$wix\light-real.exe") -and -not (Test-Path C:\Windows\SysWOW64\vbscript.dll)) {
    Move-Item "$wix\light.exe" "$wix\light-real.exe"; Copy-Item "$wix\light.exe.config" "$wix\light-real.exe.config"
    cmd.exe /c "rustc -O -o $wix\light.exe C:\sbx\in\wgwin\light-sval.rs 2>&1"
    "light.exe sostituito (aggiunge -sval: validazione ICE saltata): rustc exit $LASTEXITCODE"
  }
}

"=== chiavi usa e getta dell'updater (giusta e sbagliata) ==="
$pw = 'spike-only'
foreach ($n in 'spike', 'wrong') {
  if (-not (Test-Path "$K\$n.key")) { cmd.exe /c "cargo tauri signer generate --ci -p $pw -w $K\$n.key > $K\$n.gen.log 2>&1" }
  "$n.key.pub: " + (Get-Content "$K\$n.key.pub" -Raw).Trim()
}
$pub = (Get-Content "$K\spike.key.pub" -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content "$K\spike.key" -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $pw
$conf0 = Get-Content C:\sbx\in\src\app\src-tauri\tauri.conf.json -Raw
$cargo0 = Get-Content C:\sbx\in\src\app\src-tauri\Cargo.toml -Raw
$enc = New-Object Text.UTF8Encoding($false)

foreach ($v in '0.0.1', '0.0.2') {
  "=== build $v ==="
  [IO.File]::WriteAllText('C:\gl\app\src-tauri\tauri.conf.json', $conf0.Replace('REPLACED_AT_BUILD_TIME', $pub).Replace('"version": "0.0.1"', "`"version`": `"$v`""), $enc)
  [IO.File]::WriteAllText('C:\gl\app\src-tauri\Cargo.toml', $cargo0.Replace('version = "0.0.1"', "version = `"$v`""), $enc)
  Copy-Item "C:\sbx\in\msi\galleriad-$v.exe" C:\gl\stage\galleriad.exe -Force
  Copy-Item C:\sbx\in\msi\wireguard.dll C:\gl\stage\wireguard.dll -Force
  cmd.exe /c "cargo tauri build > $OUT\build-$v.log 2>&1"
  "cargo tauri build exit: $LASTEXITCODE"
  if ($LASTEXITCODE -ne 0 -and (Select-String -Path "$OUT\build-$v.log" -Pattern 'error code is 2738' -Quiet)) {
    "validazione ICE fallita per VBScript assente (errore 2738): log in build-$v-ice.log, si riprova senza validazione"
    Copy-Item "$OUT\build-$v.log" "$OUT\build-$v-ice.log" -Force
    Patch-Light
    cmd.exe /c "cargo tauri build > $OUT\build-$v.log 2>&1"
    "cargo tauri build exit: $LASTEXITCODE"
  }
  Get-Content "$OUT\build-$v.log" | Select-Object -Last 12
  New-Item -ItemType Directory -Force "$REL\$v" | Out-Null
  Get-ChildItem target\release\bundle\msi -Filter "*_${v}_*" -ErrorAction SilentlyContinue | ForEach-Object { Copy-Item $_.FullName "$REL\$v\" -Force }
  Get-ChildItem "$REL\$v" | ForEach-Object { "{0,10}  {1}  {2}" -f $_.Length, (Get-FileHash $_.FullName -Algorithm SHA256).Hash, $_.Name }
}

"=== firma sbagliata: lo stesso MSI 0.0.2 firmato con l'altra chiave ==="
New-Item -ItemType Directory -Force "$REL\wrong" | Out-Null
$msi2 = Get-ChildItem "$REL\0.0.2" -Filter *.msi | Select-Object -First 1
if ($msi2) {
  Copy-Item $msi2.FullName "$REL\wrong\app.msi" -Force
  Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY, Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  cmd.exe /c "cargo tauri signer sign -f $K\wrong.key -p $pw $REL\wrong\app.msi > $OUT\sign-wrong.log 2>&1"
  "signer sign exit: $LASTEXITCODE"
  Get-ChildItem "$REL\wrong" | ForEach-Object { "{0,10}  {1}  {2}" -f $_.Length, (Get-FileHash $_.FullName -Algorithm SHA256).Hash, $_.Name }
}
"build done"
