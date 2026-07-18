param(
  [switch]$UpdateBaseline,
  [switch]$SkipReference
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$output = Join-Path $root 'artifacts\visual'
New-Item -ItemType Directory -Force -Path $output | Out-Null

function Find-Playwright {
  if ($env:ITIME_PLAYWRIGHT) {
    if (-not (Test-Path -LiteralPath $env:ITIME_PLAYWRIGHT)) { throw "ITIME_PLAYWRIGHT 指向不存在的文件：$env:ITIME_PLAYWRIGHT" }
    $python = Join-Path (Split-Path -Parent (Split-Path -Parent $env:ITIME_PLAYWRIGHT)) 'python.exe'
    return @{ Kind = 'exe'; Value = $env:ITIME_PLAYWRIGHT; Python = $(if (Test-Path -LiteralPath $python) { $python } else { $null }) }
  }
  $command = Get-Command playwright -ErrorAction SilentlyContinue
  if ($command) {
    $python = Join-Path (Split-Path -Parent (Split-Path -Parent $command.Source)) 'python.exe'
    return @{ Kind = 'exe'; Value = $command.Source; Python = $(if (Test-Path -LiteralPath $python) { $python } else { $null }) }
  }
  $pythonLauncher = Get-Command py -ErrorAction SilentlyContinue
  if ($pythonLauncher) {
    & $pythonLauncher.Source -m playwright --version *> $null
    if ($LASTEXITCODE -eq 0) { return @{ Kind = 'module'; Value = $pythonLauncher.Source; Python = $pythonLauncher.Source } }
  }
  if ($env:PLAYWRIGHT_BROWSERS_PATH) {
    $sharedRoot = Split-Path -Parent $env:PLAYWRIGHT_BROWSERS_PATH
    $shared = Get-ChildItem -LiteralPath $sharedRoot -Filter playwright.exe -File -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($shared) {
      $python = Join-Path (Split-Path -Parent (Split-Path -Parent $shared.FullName)) 'python.exe'
      return @{ Kind = 'exe'; Value = $shared.FullName; Python = $(if (Test-Path -LiteralPath $python) { $python } else { $null }) }
    }
  }
  throw '未找到共享 Playwright。请在现有共享 Python 环境安装 playwright，并将可执行文件加入 PATH，或设置 ITIME_PLAYWRIGHT=<playwright.exe 的完整路径>。项目不会自行安装第二份 Playwright。'
}

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

$runtime = Find-Playwright
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
