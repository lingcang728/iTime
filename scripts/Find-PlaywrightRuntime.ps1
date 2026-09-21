# Shared Playwright runtime discovery for visual and native smoke gates.
# Dot-source this file, then call Find-PlaywrightRuntime.
#
# Resolution order (no machine-specific hardcoded paths):
#   1. $env:ITIME_PLAYWRIGHT  -> explicit path to a playwright.exe shim
#   2. Get-Command playwright -> any playwright.exe already on PATH
#   3. py -m playwright       -> Python launcher with the module installed
#   4. python -m playwright   -> PATH python with the module installed
#   5. $env:PLAYWRIGHT_BROWSERS_PATH -> scan sibling dirs for playwright.exe
#
# Returns @{ Kind = 'exe'|'module'; Value = <playwright.exe or launcher>; Python = <python.exe or launcher or $null> }
# Python is $null when only the CLI shim was found and no interpreter could be
# located next to it (the caller decides whether it needs Python).

function Find-PlaywrightRuntime {
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
  foreach ($launcherName in @('py', 'python')) {
    $launcher = Get-Command $launcherName -ErrorAction SilentlyContinue
    if ($launcher) {
      & $launcher.Source -m playwright --version *> $null
      if ($LASTEXITCODE -eq 0) { return @{ Kind = 'module'; Value = $launcher.Source; Python = $launcher.Source } }
    }
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
