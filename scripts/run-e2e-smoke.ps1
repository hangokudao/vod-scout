param(
  [Parameter(Mandatory = $true)][string]$SnapshotPath,
  [Parameter(Mandatory = $true)][string]$DataDirectory,
  [ValidateSet("quick", "range", "full")][string]$Mode = "full",
  [int]$Port = 9225,
  [switch]$Long,
  [switch]$VerifyDelete
)

$ErrorActionPreference = "Stop"
$snapshot = Get-Content -LiteralPath $SnapshotPath -Raw -Encoding utf8 | ConvertFrom-Json
$source = if ($snapshot.acquiredMediaPath) { $snapshot.acquiredMediaPath } else { $snapshot.sourceLabel }
if (-not (Test-Path -LiteralPath $source)) { throw "E2E 입력 파일을 찾을 수 없습니다." }

$env:VOD_SCOUT_HEADLESS_E2E = "1"
$env:VOD_SCOUT_E2E_DATA_DIR = $DataDirectory
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$app = Start-Process -FilePath "src-tauri\target\release\vod-scout.exe" -WindowStyle Hidden -PassThru
try {
  $ready = $false
  for ($index = 0; $index -lt 40; $index += 1) {
    try {
      Invoke-RestMethod "http://127.0.0.1:$Port/json" -TimeoutSec 1 | Out-Null
      $ready = $true
      break
    } catch {
      Start-Sleep -Milliseconds 250
    }
  }
  if (-not $ready) { throw "headless CDP가 열리지 않았습니다." }
  $arguments = @("scripts/e2e-local-cdp.mjs", $source, "$Port", "--mode", $Mode)
  if ($Long) { $arguments += "--long" }
  if ($VerifyDelete) { $arguments += "--verify-delete" }
  & node @arguments
  if ($LASTEXITCODE -ne 0) { throw "E2E 검증 실패: exit $LASTEXITCODE" }
} finally {
  Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
}
