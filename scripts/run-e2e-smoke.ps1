param(
  [Parameter(Mandatory = $true)][string]$SnapshotPath,
  [Parameter(Mandatory = $true)][string]$DataDirectory,
  [ValidateSet("quick", "range", "full")][string]$Mode = "full",
  [int]$Port = 9225,
  [switch]$Long,
  [switch]$VerifyDelete,
  [string]$AppPath = "src-tauri\target\release\vod-scout.exe",
  [string]$ScreenshotPath,
  [string]$EvidencePath
)

$ErrorActionPreference = "Stop"

function Resolve-E2eDataDirectory([string]$RequestedDataDirectory) {
  if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
    throw "E2E 데이터 경로를 확인하지 못했습니다: LOCALAPPDATA가 없습니다."
  }
  if ([string]::IsNullOrWhiteSpace($RequestedDataDirectory)) {
    throw "E2E 데이터 경로를 지정해야 합니다."
  }

  $requestedLeaf = Split-Path -Leaf $RequestedDataDirectory.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
  if ([string]::IsNullOrWhiteSpace($requestedLeaf) -or $requestedLeaf -in @(".", "..")) {
    throw "E2E 데이터 경로의 마지막 폴더 이름이 올바르지 않습니다."
  }

  $safeLeaf = $requestedLeaf -replace "[^A-Za-z0-9._-]", "-"
  if ([string]::IsNullOrWhiteSpace($safeLeaf)) {
    throw "E2E 데이터 경로의 마지막 폴더 이름이 올바르지 않습니다."
  }
  if (-not $safeLeaf.StartsWith("e2e-", [StringComparison]::OrdinalIgnoreCase)) {
    $safeLeaf = "e2e-$safeLeaf"
  }

  $appLocalDataRoot = Join-Path $env:LOCALAPPDATA "com.vodscout.app"
  $resolved = [IO.Path]::GetFullPath((Join-Path $appLocalDataRoot $safeLeaf))
  $rootPrefix = [IO.Path]::GetFullPath($appLocalDataRoot).TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
  if (-not $resolved.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase) -or $resolved.Substring($rootPrefix.Length) -notmatch "^e2e-[A-Za-z0-9._-]+$") {
    throw "E2E 데이터 경로는 com.vodscout.app 아래 e2e-* 폴더여야 합니다."
  }
  return $resolved
}

$snapshot = Get-Content -LiteralPath $SnapshotPath -Raw -Encoding utf8 | ConvertFrom-Json
$source = if ($snapshot.acquiredMediaPath) { $snapshot.acquiredMediaPath } else { $snapshot.sourceLabel }
$isYoutube = $source -match '^https?://(www\.)?(youtube\.com/watch\?v=|youtu\.be/)'
if (-not $isYoutube -and -not (Test-Path -LiteralPath $source)) { throw "E2E 입력 파일을 찾을 수 없습니다." }
if (-not (Test-Path -LiteralPath $AppPath)) { throw "VOD Scout 앱을 찾을 수 없습니다: $AppPath" }

function Stop-ProcessTree([int]$ProcessId) {
  $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $ProcessId" | Select-Object -ExpandProperty ProcessId)
  foreach ($childPid in $children) {
    Stop-ProcessTree -ProcessId $childPid
  }
  Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
}

$env:VOD_SCOUT_HEADLESS_E2E = "1"
$resolvedDataDirectory = Resolve-E2eDataDirectory $DataDirectory
$env:VOD_SCOUT_E2E_DATA_DIR = $resolvedDataDirectory
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$app = Start-Process -FilePath $AppPath -WindowStyle Hidden -PassThru
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
  if ($isYoutube) { $arguments += "--youtube" }
  if ($Long) { $arguments += "--long" }
  if ($VerifyDelete) { $arguments += "--verify-delete" }
  if ($ScreenshotPath) { $arguments += @("--screenshot", $ScreenshotPath) }
  if ($EvidencePath) { $arguments += @("--evidence-dir", $EvidencePath) }
  & node @arguments
  if ($LASTEXITCODE -ne 0) { throw "E2E 검증 실패: exit $LASTEXITCODE" }
} finally {
  Stop-ProcessTree -ProcessId $app.Id
}
