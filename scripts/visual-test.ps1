param(
  [switch]$UpdateBaseline,
  [switch]$SkipReference,
  # Native mode (default) exercises the packaged release\iTime.exe over real
  # WebView2 + IPC. -Dev keeps the old Chrome+Vite path for local layout
  # iteration only — it must never be used as the release gate.
  [switch]$Dev,
  [string]$Executable = 'release\iTime.exe'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

if (-not $Dev) {
  # The visual release gate runs on the real packaged application: the native
  # smoke launches $Executable with an isolated environment, captures
  # native-wide-<page>.png screenshots over the WebView2 CDP endpoint and
  # compares them against tests\visual\native-baseline (see
  # native-release-smoke.ps1 + compare-visual.mjs --native).
  $smoke = Join-Path $PSScriptRoot 'native-release-smoke.ps1'
  $exePath = if ([System.IO.Path]::IsPathRooted($Executable)) { $Executable } else { Join-Path $root $Executable }
  if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
    throw "缺少打包产物 $Executable；真实应用视觉门禁需要 release\iTime.exe，请先运行 npm run package:release（本地布局迭代可用 -Dev）。"
  }
  if ($UpdateBaseline) {
    & $smoke -Executable $Executable -UpdaterCheck off -UpdateVisualBaseline
    if ($LASTEXITCODE -ne 0) { throw '真实 EXE 原生验收失败，未更新视觉基线。' }
  } else {
    & $smoke -Executable $Executable -UpdaterCheck off
    if ($LASTEXITCODE -ne 0) { throw '真实 EXE 视觉与功能验收失败。' }
  }
  exit 0
}

# ------------------------- dev-only Vite path -------------------------
# Chrome + Vite only proves layout inside a dev server; it says nothing about
# the shipped WebView2 shell, IPC payloads or bundled assets. Keep it for
# component-level iteration behind -Dev.
$output = Join-Path $root 'artifacts\visual'
New-Item -ItemType Directory -Force -Path $output | Out-Null

. (Join-Path $PSScriptRoot 'Find-PlaywrightRuntime.ps1')

function Invoke-Playwright([hashtable]$runtime, [string[]]$arguments) {
  if ($arguments[0] -eq 'screenshot' -and -not ($arguments -contains '--channel')) {
    $chromePaths = @(
      "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
      "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe",
      "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe"
    )
    if ($chromePaths | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1) {
      $arguments = @('screenshot', '--channel', 'chrome') + $arguments[1..($arguments.Length - 1)]
    } elseif (Test-Path -LiteralPath "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") {
      $arguments = @('screenshot', '--channel', 'msedge') + $arguments[1..($arguments.Length - 1)]
    }
  }
  if ($runtime.Kind -eq 'module') { & $runtime.Value -m playwright @arguments }
  else { & $runtime.Value @arguments }
  if ($LASTEXITCODE -ne 0) { throw "Playwright 命令失败：$($arguments -join ' ')" }
}

$runtime = Find-PlaywrightRuntime
$viteEntry = Join-Path $root 'node_modules\vite\bin\vite.js'
if (-not (Test-Path -LiteralPath $viteEntry)) { throw '缺少 node_modules。请先运行 npm install。' }
$server = Start-Process -FilePath (Get-Command node).Source -ArgumentList @($viteEntry, '--host', '127.0.0.1', '--port', '1420', '--strictPort') -WorkingDirectory $root -WindowStyle Hidden -PassThru
try {
  $ready = $false
  for ($attempt = 0; $attempt -lt 60; $attempt += 1) {
    try {
      $response = Invoke-WebRequest -UseBasicParsing -Uri 'http://127.0.0.1:1420' -TimeoutSec 1
      if ($response.StatusCode -eq 200) { $ready = $true; break }
    } catch {}
    Start-Sleep -Milliseconds 250
  }
  if (-not $ready) { throw 'Vite 视觉测试服务器未在 15 秒内就绪。' }

  if ($runtime.Python) {
    & $runtime.Python (Join-Path $PSScriptRoot 'visual_harness.py') 'http://127.0.0.1:1420'
    if ($LASTEXITCODE -ne 0) { throw '视觉布局、交互、DPI 或最小窗口矩阵未通过，请查看 artifacts\visual\layout-report.json。' }
  } else {
    throw '已发现 Playwright CLI，但未找到其 Python 运行时。请设置 ITIME_PLAYWRIGHT 指向共享 Python 环境中的 playwright.exe，以运行完整视觉门禁。'
  }

  if ($UpdateBaseline) {
    $baseline = Join-Path $root 'tests\visual\baseline'
    New-Item -ItemType Directory -Force -Path $baseline | Out-Null
    @('wide-*.png', 'dark-*.png') | ForEach-Object {
      Get-ChildItem -LiteralPath $output -Filter $_ | Copy-Item -Destination $baseline -Force
    }
  }
  $compareArgs = @('scripts/compare-visual.mjs')
  if ($SkipReference) { $compareArgs += '--skip-reference' }
  & (Get-Command node).Source @compareArgs
  if ($LASTEXITCODE -ne 0) { throw '视觉差异超出项目阈值，请查看 artifacts\visual\report.json 与差异图。' }
} finally {
  if ($server -and -not $server.HasExited) { Stop-Process -Id $server.Id -Force }
}
