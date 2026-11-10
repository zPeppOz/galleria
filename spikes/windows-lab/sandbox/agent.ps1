# Agent loop: esegue in ordine gli script C:\sbx\in\jobs\*.ps1 non ancora eseguiti; output in C:\sbx\out\jobs\
# Il file C:\sbx\in\jobs\STOP spegne l'ospite (la Sandbox si chiude da sola).
$ErrorActionPreference = 'Continue'
$in = 'C:\sbx\in\jobs'; $out = 'C:\sbx\out\jobs'
New-Item -ItemType Directory -Force $out | Out-Null
"agent up $(Get-Date -Format o) $(whoami)" | Set-Content C:\sbx\out\agent-up.txt
while ($true) {
  if (Test-Path C:\sbx\in\jobs\STOP) { "stopping" | Set-Content C:\sbx\out\agent-stop.txt; shutdown.exe /s /t 0; break }
  Get-ChildItem $in -Filter *.ps1 -ErrorAction SilentlyContinue | Sort-Object Name | ForEach-Object {
    $n = $_.BaseName
    if (-not (Test-Path "$out\$n.done")) {
      "started $(Get-Date -Format o)" | Set-Content "$out\$n.started"
      $p = Start-Process powershell.exe -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File',$_.FullName -PassThru -WindowStyle Hidden -RedirectStandardOutput "$out\$n.log" -RedirectStandardError "$out\$n.err"; $p.WaitForExit()
      "exit $($p.ExitCode) $(Get-Date -Format o)" | Set-Content "$out\$n.done"
    }
  }
  Start-Sleep -Milliseconds 700
}
