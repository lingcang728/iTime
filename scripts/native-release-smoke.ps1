param(
  [string]$Executable = 'release\iTime.exe',
  [string]$OutputDirectory = 'artifacts\native-qa'
)

$ErrorActionPreference = 'Stop'

$root = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$rootPrefix = $root.TrimEnd('\') + '\'
$executablePath = if ([System.IO.Path]::IsPathRooted($Executable)) {
  [System.IO.Path]::GetFullPath($Executable)
} else {
  [System.IO.Path]::GetFullPath((Join-Path $root $Executable))
}
$outputPath = if ([System.IO.Path]::IsPathRooted($OutputDirectory)) {
  throw '原生 QA 输出目录必须使用仓库相对路径。'
} else {
  [System.IO.Path]::GetFullPath((Join-Path $root $OutputDirectory))
}
if (-not $executablePath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw '原生 QA 只允许运行当前仓库内的 EXE。'
}
if (-not $outputPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw '原生 QA 输出目录必须位于当前仓库内。'
}
if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
  throw "缺少待验收 EXE：$executablePath"
}

$python = 'G:\python\python.exe'
$playwright = 'G:\python\Scripts\playwright.exe'
if (-not (Test-Path -LiteralPath $python -PathType Leaf) -or
    -not (Test-Path -LiteralPath $playwright -PathType Leaf)) {
  throw '未找到本机共享 Python Playwright；禁止在项目内重复安装。'
}

$otherInstances = @(Get-Process -Name iTime -ErrorAction SilentlyContinue)
if ($otherInstances.Count -gt 0) {
  $paths = @($otherInstances | ForEach-Object { $_.Path }) -join '；'
  throw "原生 QA 前必须关闭现有 iTime 实例，避免单实例重定向：$paths"
}

function Get-ShellSnapshot {
  $appPaths = 'Software\Microsoft\Windows\CurrentVersion\App Paths\iTime.exe'
  $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($appPaths)
  $registry = if ($null -eq $key) {
    $null
  } else {
    try {
      $values = [ordered]@{}
      foreach ($name in @($key.GetValueNames() | Sort-Object)) {
        $label = if ([string]::IsNullOrEmpty($name)) { '(default)' } else { $name }
        $values[$label] = [ordered]@{
          kind = $key.GetValueKind($name).ToString()
          value = [string]$key.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        }
      }
      $values
    } finally {
      $key.Dispose()
    }
  }

  $startMenuRoot = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
  $shortcuts = @(
    Get-ChildItem -LiteralPath $startMenuRoot -Filter '*iTime*.lnk' -Recurse -File -ErrorAction SilentlyContinue |
      Sort-Object FullName |
      ForEach-Object {
        [ordered]@{
          relativePath = [System.IO.Path]::GetRelativePath($startMenuRoot, $_.FullName)
          length = $_.Length
          sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        }
      }
  )
  $runKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Run')
  $autostart = if ($null -eq $runKey) {
    $null
  } else {
    try {
      $runKey.GetValue('iTime', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally {
      $runKey.Dispose()
    }
  }
  [ordered]@{
    appPaths = $registry
    shortcuts = $shortcuts
    autostart = $autostart
  }
}

function Write-Utf8NoBom {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Content
  )
  $encoding = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

$transaction = [guid]::NewGuid().ToString('N')
$runtimePath = Join-Path $outputPath "runtime-$transaction"
$runtimePrefix = $outputPath.TrimEnd('\') + '\'
if (-not $runtimePath.StartsWith($runtimePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw '拒绝使用越界的原生 QA 临时目录。'
}
$localAppData = Join-Path $runtimePath 'LocalAppData'
$roamingAppData = Join-Path $runtimePath 'AppData\Roaming'
$tempPath = Join-Path $runtimePath 'Temp'
@($outputPath, $localAppData, $roamingAppData, $tempPath) | ForEach-Object {
  New-Item -ItemType Directory -Force -Path $_ | Out-Null
}

$reportPath = Join-Path $outputPath 'report.json'
$screenshotPath = Join-Path $outputPath 'settings.png'
$devToolsPortPath = Join-Path $runtimePath 'WebView2\EBWebView\DevToolsActivePort'
$pythonScript = Join-Path $PSScriptRoot 'native-release-smoke.py'
$before = Get-ShellSnapshot
$beforeJson = $before | ConvertTo-Json -Depth 8 -Compress
$process = $null
$port = 0

try {
  $childEnvironment = @{
    LOCALAPPDATA = $localAppData
    APPDATA = $roamingAppData
    TEMP = $tempPath
    TMP = $tempPath
    ITIME_NATIVE_QA = '1'
    WEBVIEW2_USER_DATA_FOLDER = (Join-Path $runtimePath 'WebView2')
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=0'
  }
  $process = Start-Process `
    -FilePath $executablePath `
    -WorkingDirectory (Split-Path -Parent $executablePath) `
    -WindowStyle Hidden `
    -Environment $childEnvironment `
    -PassThru

  $deadline = (Get-Date).AddSeconds(30)
  $version = $null
  do {
    $process.Refresh()
    if ($process.HasExited) {
      throw "真实 EXE 在 WebView2 就绪前退出，退出码：$($process.ExitCode)"
    }
    if (Test-Path -LiteralPath $devToolsPortPath -PathType Leaf) {
      $portLine = Get-Content -LiteralPath $devToolsPortPath -TotalCount 1 -ErrorAction SilentlyContinue
      $candidatePort = 0
      if ([int]::TryParse($portLine, [ref]$candidatePort) -and $candidatePort -gt 0) {
        $port = $candidatePort
        try {
          $version = Invoke-RestMethod -Uri "http://127.0.0.1:$port/json/version" -TimeoutSec 2
        } catch {
          $version = $null
        }
      }
    }
    if ($null -eq $version) { Start-Sleep -Milliseconds 400 }
  } while ($null -eq $version -and (Get-Date) -lt $deadline)
  if ($null -eq $version) { throw '真实 WebView2 临时 CDP 端点未在 30 秒内就绪。' }

  & $python `
    $pythonScript `
    --cdp-url "http://127.0.0.1:$port" `
    --host-pid $process.Id `
    --isolated-root $runtimePath `
    --report $reportPath `
    --screenshot $screenshotPath
  if ($LASTEXITCODE -ne 0) { throw '真实 EXE 原生功能验收失败。' }

  $after = Get-ShellSnapshot
  $afterJson = $after | ConvertTo-Json -Depth 8 -Compress
  if ($beforeJson -cne $afterJson) {
    throw '便携版原生冒烟修改了 App Paths、开始菜单或自启动入口。'
  }

  $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
  $report | Add-Member -NotePropertyName executable -NotePropertyValue ([ordered]@{
    fileName = [System.IO.Path]::GetFileName($executablePath)
    sizeBytes = (Get-Item -LiteralPath $executablePath).Length
    sha256 = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash
    productVersion = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($executablePath).ProductVersion
  })
  $report | Add-Member -NotePropertyName webView2 -NotePropertyValue ([ordered]@{
    browser = $version.Browser
    protocolVersion = $version.'Protocol-Version'
    ephemeralDebugPort = $true
    debugPortClosedAfterQa = $false
  })
  $report | Add-Member -NotePropertyName portableShellIsolation -NotePropertyValue ([ordered]@{
    appPathsUnchanged = $true
    startMenuUnchanged = $true
    autostartUnchanged = $true
  })
  Write-Utf8NoBom -Path $reportPath -Content ($report | ConvertTo-Json -Depth 12)
} finally {
  if ($process) {
    $scoped = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
    if ($scoped -and $scoped.Path -eq $executablePath) {
      Stop-Process -Id $process.Id -Force
      Wait-Process -Id $process.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
  }
  Start-Sleep -Milliseconds 400
  if ($port -gt 0) {
    $debugListeners = @(Get-NetTCPConnection -State Listen -LocalPort $port -ErrorAction SilentlyContinue)
    if ($debugListeners.Count -gt 0) {
      throw "原生 QA 结束后调试端口 $port 仍在监听。"
    }
  }
  if (Test-Path -LiteralPath $runtimePath) {
    $resolvedRuntime = [System.IO.Path]::GetFullPath($runtimePath)
    if (-not $resolvedRuntime.StartsWith($runtimePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
      throw '拒绝删除越界的原生 QA 临时目录。'
    }
    Remove-Item -LiteralPath $resolvedRuntime -Recurse -Force
  }
}

$finalReport = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
$finalReport.webView2.debugPortClosedAfterQa = $true
Write-Utf8NoBom -Path $reportPath -Content ($finalReport | ConvertTo-Json -Depth 12)
$finalReport
