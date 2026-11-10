# Server HTTP minimo dentro la Sandbox, solo su 127.0.0.1:8765, per latest.json e gli MSI dell'updater (criterio 1.7)
$l = New-Object Net.HttpListener; $l.Prefixes.Add('http://127.0.0.1:8765/'); $l.Start()
while ($l.IsListening) {
  $c = $l.GetContext(); $f = Join-Path 'C:\gl\www' ($c.Request.Url.AbsolutePath.TrimStart('/'))
  if (Test-Path $f -PathType Leaf) { $b = [IO.File]::ReadAllBytes($f); $c.Response.ContentLength64 = $b.Length; $c.Response.OutputStream.Write($b, 0, $b.Length) } else { $c.Response.StatusCode = 404 }
  Add-Content C:\gl\log\httpd.log "$((Get-Date).ToUniversalTime().ToString('s'))Z $($c.Request.HttpMethod) $($c.Request.Url.AbsolutePath) -> $($c.Response.StatusCode) $($c.Response.ContentLength64) byte"
  $c.Response.Close()
}
