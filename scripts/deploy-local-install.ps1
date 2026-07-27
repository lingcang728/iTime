param(
  [string]$ReleaseExecutable = 'release\iTime.exe',
  [string]$InstallDirectory
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$rootPrefix = [System.IO.Path]::GetFullPath($root).TrimEnd('\') + '\'

function Get-Sha256 {
  param([Parameter(Mandatory = $true)][string]$Path)
  $stream = [System.IO.File]::OpenRead($Path)
  try {
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
      return (($algorithm.ComputeHash($stream) | ForEach-Object { $_.ToString('X2') }) -join '')
    } finally {
      $algorithm.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
}

function Copy-WithRetry {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  for ($attempt = 1; $attempt -le 15; $attempt += 1) {
    try {
      Copy-Item -LiteralPath $Source -Destination $Destination -Force
      return
    } catch {
      if ($attempt -eq 15) { throw }
      Start-Sleep -Milliseconds 400
    }
  }
}

function Resolve-DesktopShortcut {
  $desktop = [Environment]::GetFolderPath('Desktop')
  $link = Join-Path $desktop 'iTime.lnk'
  if (-not (Test-Path -LiteralPath $link -PathType Leaf)) {
    return $null
  }
  $shell = New-Object -ComObject WScript.Shell
  try {
    $shortcut = $shell.CreateShortcut($link)
    return [pscustomobject]@{
      Path = $link
      TargetPath = [string]$shortcut.TargetPath
      WorkingDirectory = [string]$shortcut.WorkingDirectory
    }
  } finally {
    [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell)
  }
}

function Set-DesktopShortcut {
  param(
    [Parameter(Mandatory = $true)][string]$TargetPath,
    [Parameter(Mandatory = $true)][string]$WorkingDirectory
  )

  $desktop = [Environment]::GetFolderPath('Desktop')
  $link = Join-Path $desktop 'iTime.lnk'
  $shell = New-Object -ComObject WScript.Shell
  try {
    $shortcut = $shell.CreateShortcut($link)
    $shortcut.TargetPath = $TargetPath
    $shortcut.WorkingDirectory = $WorkingDirectory
    $shortcut.Description = 'iTime'
    if (Test-Path -LiteralPath $TargetPath -PathType Leaf) {
      $shortcut.IconLocation = "$TargetPath,0"
    }
    $shortcut.Save()
  } finally {
    [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell)
  }
  return $link
}

function Stop-InstalledITime {
  param([Parameter(Mandatory = $true)][string]$InstalledExecutable)

  $targetFull = [System.IO.Path]::GetFullPath($InstalledExecutable)
  $running = @(Get-Process -Name 'iTime', 'itime' -ErrorAction SilentlyContinue | Where-Object {
      try {
        if (-not $_.Path) { return $true }
        return ([System.IO.Path]::GetFullPath($_.Path)).Equals(
          $targetFull,
          [System.StringComparison]::OrdinalIgnoreCase
        )
      } catch {
        return $true
      }
    })

  if ($running.Count -eq 0) { return 0 }

  foreach ($proc in $running) {
    try {
      $proc.CloseMainWindow() | Out-Null
    } catch {
      # Ignore windows that cannot receive close messages.
    }
  }
  Start-Sleep -Milliseconds 800

  $still = @(Get-Process -Name 'iTime', 'itime' -ErrorAction SilentlyContinue | Where-Object {
      try {
        if (-not $_.Path) { return $true }
        return ([System.IO.Path]::GetFullPath($_.Path)).Equals(
          $targetFull,
          [System.StringComparison]::OrdinalIgnoreCase
        )
      } catch {
        return $true
      }
    })
  foreach ($proc in $still) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  }
  Start-Sleep -Milliseconds 400
  return $running.Count
}

$releasePath = if ([System.IO.Path]::IsPathRooted($ReleaseExecutable)) {
  [System.IO.Path]::GetFullPath($ReleaseExecutable)
} else {
  [System.IO.Path]::GetFullPath((Join-Path $root $ReleaseExecutable))
}
if (-not $releasePath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw '本地安装同步只允许使用当前仓库 release 目录内的 EXE。'
}
if (-not (Test-Path -LiteralPath $releasePath -PathType Leaf)) {
  throw "缺少本轮发布可执行文件：$releasePath"
}

$defaultInstallDir = Join-Path $env:LOCALAPPDATA 'iTime'
$shortcut = Resolve-DesktopShortcut
if (-not [string]::IsNullOrWhiteSpace($InstallDirectory)) {
  $installDir = [System.IO.Path]::GetFullPath($InstallDirectory)
} elseif ($null -ne $shortcut -and -not [string]::IsNullOrWhiteSpace($shortcut.TargetPath)) {
  $installDir = [System.IO.Path]::GetDirectoryName([System.IO.Path]::GetFullPath($shortcut.TargetPath))
} else {
  $installDir = $defaultInstallDir
}

if ([string]::IsNullOrWhiteSpace($installDir)) {
  throw '无法解析本机 iTime 安装目录。'
}

$installedExecutable = Join-Path $installDir 'itime.exe'
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

$stopped = Stop-InstalledITime -InstalledExecutable $installedExecutable
$sourceHash = Get-Sha256 -Path $releasePath
$transaction = [guid]::NewGuid().ToString('N')
$staged = Join-Path $installDir ".itime-$transaction.exe.new"
$backup = Join-Path $installDir ".itime-$transaction.exe.bak"
$hadInstalled = Test-Path -LiteralPath $installedExecutable -PathType Leaf

try {
  Copy-WithRetry -Source $releasePath -Destination $staged
  if ((Get-Sha256 -Path $staged) -ne $sourceHash) {
    throw '安装目录暂存文件校验失败。'
  }
  if ($hadInstalled) {
    Copy-WithRetry -Source $installedExecutable -Destination $backup
  }
  Copy-WithRetry -Source $staged -Destination $installedExecutable
  $installedHash = Get-Sha256 -Path $installedExecutable
  if ($installedHash -ne $sourceHash) {
    throw "安装目录 EXE 与 release 不一致：installed=$installedHash release=$sourceHash"
  }
} catch {
  if ((Test-Path -LiteralPath $backup -PathType Leaf) -and $hadInstalled) {
    Copy-WithRetry -Source $backup -Destination $installedExecutable
  }
  throw
} finally {
  Remove-Item -LiteralPath $staged -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue
}

$linkPath = Set-DesktopShortcut -TargetPath $installedExecutable -WorkingDirectory $installDir
$verifiedShortcut = Resolve-DesktopShortcut
if ($null -eq $verifiedShortcut) {
  throw '桌面快捷方式写入后不存在。'
}
$shortcutTarget = [System.IO.Path]::GetFullPath($verifiedShortcut.TargetPath)
$expectedTarget = [System.IO.Path]::GetFullPath($installedExecutable)
if (-not $shortcutTarget.Equals($expectedTarget, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "桌面快捷方式未指向最新安装包：got=$shortcutTarget expected=$expectedTarget"
}
if (-not (Test-Path -LiteralPath $shortcutTarget -PathType Leaf)) {
  throw "桌面快捷方式目标不存在：$shortcutTarget"
}

[PSCustomObject]@{
  ReleaseExecutable = $releasePath
  InstalledExecutable = $installedExecutable
  DesktopShortcut = $linkPath
  ShortcutTarget = $shortcutTarget
  Sha256 = $sourceHash
  StoppedProcesses = $stopped
  Bytes = (Get-Item -LiteralPath $installedExecutable).Length
  Modified = (Get-Item -LiteralPath $installedExecutable).LastWriteTime.ToString('o')
} | Format-List
Write-Host "Local install updated. Desktop shortcut -> $shortcutTarget"
