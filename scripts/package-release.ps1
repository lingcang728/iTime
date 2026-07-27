param(
  [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'src-tauri\Cargo.toml'
$tauriConfig = Join-Path $root 'src-tauri\tauri.conf.json'
$packageJson = Join-Path $root 'package.json'
$releaseDirectory = if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
  Join-Path $root 'release'
} else {
  if ([System.IO.Path]::IsPathRooted($OutputDirectory)) {
    throw '自定义发布输出目录必须使用仓库相对路径。'
  }
  [System.IO.Path]::GetFullPath((Join-Path $root $OutputDirectory))
}
$rootPrefix = [System.IO.Path]::GetFullPath($root).TrimEnd('\') + '\'
if (-not $releaseDirectory.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw '发布输出目录必须位于当前仓库内。'
}
$releaseManifest = Join-Path $releaseDirectory 'release-manifest.json'
$releaseVerifier = Join-Path $PSScriptRoot 'verify-release-manifest.ps1'

function Copy-WithRetry {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  for ($attempt = 1; $attempt -le 10; $attempt += 1) {
    try {
      Copy-Item -LiteralPath $Source -Destination $Destination -Force
      return
    } catch {
      if ($attempt -eq 10) { throw }
      Start-Sleep -Milliseconds 500
    }
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

Push-Location $root
try {
  & npm run verify:full
  if ($LASTEXITCODE -ne 0) { throw '完整验证门禁失败，拒绝打包发布。' }

  $metadataRaw = & cargo metadata --format-version 1 --no-deps --manifest-path $manifest
  if ($LASTEXITCODE -ne 0) { throw '无法解析 Cargo 元数据。' }
  $metadata = $metadataRaw | ConvertFrom-Json
  $targetDirectory = $metadata.target_directory
  $tauriVersion = (Get-Content -Raw -LiteralPath $tauriConfig | ConvertFrom-Json).version
  $npmVersion = (Get-Content -Raw -LiteralPath $packageJson | ConvertFrom-Json).version
  $manifestPath = (Resolve-Path -LiteralPath $manifest).Path
  $cargoPackage = $metadata.packages | Where-Object { [System.IO.Path]::GetFullPath($_.manifest_path) -eq $manifestPath } | Select-Object -First 1
  if (-not $cargoPackage) { throw 'Cargo 元数据中缺少 iTime 包。' }
  if ($npmVersion -ne $tauriVersion -or $cargoPackage.version -ne $tauriVersion) {
    throw "版本不一致：npm=$npmVersion, tauri=$tauriVersion, cargo=$($cargoPackage.version)"
  }
  $gitCommit = (& git rev-parse HEAD).Trim()
  if ($LASTEXITCODE -ne 0 -or -not $gitCommit) { throw '无法读取当前 Git commit。' }
  $sourceDirty = [bool]((& git status --porcelain --untracked-files=no) -join '')

  $sourceExecutable = Join-Path $targetDirectory 'release\itime.exe'
  $bundleDirectory = Join-Path $targetDirectory 'release\bundle\nsis'
  $expectedSetupName = "iTime_${tauriVersion}_x64-setup.exe"

  Remove-Item -LiteralPath $sourceExecutable -Force -ErrorAction SilentlyContinue
  if (Test-Path -LiteralPath $bundleDirectory) {
    Get-ChildItem -LiteralPath $bundleDirectory -Filter "iTime_${tauriVersion}_*-setup.exe" -File -ErrorAction SilentlyContinue |
      Remove-Item -Force
  }

  $startedAt = Get-Date
  & npm run tauri:build
  if ($LASTEXITCODE -ne 0) { throw 'Tauri 打包失败。' }

  $sourceSetup = Get-ChildItem -LiteralPath $bundleDirectory -Filter $expectedSetupName -File | Select-Object -First 1
  if (-not (Test-Path -LiteralPath $sourceExecutable)) { throw '本轮构建没有生成可直接运行的 iTime.exe。' }
  if (-not $sourceSetup) { throw "本轮构建没有生成 $expectedSetupName。" }
  $sourceExecutableFile = Get-Item -LiteralPath $sourceExecutable
  if ($sourceExecutableFile.LastWriteTime -lt $startedAt -or $sourceSetup.LastWriteTime -lt $startedAt) {
    throw '检测到旧发布产物，拒绝同步 release。'
  }

  New-Item -ItemType Directory -Force -Path $releaseDirectory | Out-Null
  $destinationExecutable = Join-Path $releaseDirectory 'iTime.exe'
  $destinationSetup = Join-Path $releaseDirectory $expectedSetupName
  $transaction = [guid]::NewGuid().ToString('N')
  $stagedExecutable = Join-Path $releaseDirectory ".iTime-$transaction.exe.new"
  $stagedSetup = Join-Path $releaseDirectory ".iTime-setup-$transaction.exe.new"
  $backupExecutable = Join-Path $releaseDirectory ".iTime-$transaction.exe.bak"
  $backupSetup = Join-Path $releaseDirectory ".iTime-setup-$transaction.exe.bak"
  $stagedManifest = Join-Path $releaseDirectory ".release-manifest-$transaction.json.new"
  $backupManifest = Join-Path $releaseDirectory ".release-manifest-$transaction.json.bak"
  $hadDestinationExecutable = Test-Path -LiteralPath $destinationExecutable
  $hadDestinationSetup = Test-Path -LiteralPath $destinationSetup
  $hadReleaseManifest = Test-Path -LiteralPath $releaseManifest
  $wroteDestinationExecutable = $false
  $wroteDestinationSetup = $false
  $wroteReleaseManifest = $false
  $staleExecutableBackups = @()

  try {
    Copy-WithRetry -Source $sourceExecutable -Destination $stagedExecutable
    Copy-WithRetry -Source $sourceSetup.FullName -Destination $stagedSetup
    if ((Get-Sha256 -Path $sourceExecutable) -ne (Get-Sha256 -Path $stagedExecutable)) {
      throw '可执行文件暂存校验失败。'
    }
    if ((Get-Sha256 -Path $sourceSetup.FullName) -ne (Get-Sha256 -Path $stagedSetup)) {
      throw '安装包暂存校验失败。'
    }

    if ($hadDestinationExecutable) { Copy-WithRetry -Source $destinationExecutable -Destination $backupExecutable }
    if ($hadDestinationSetup) { Copy-WithRetry -Source $destinationSetup -Destination $backupSetup }
    if ($hadReleaseManifest) { Copy-WithRetry -Source $releaseManifest -Destination $backupManifest }
    Copy-WithRetry -Source $stagedExecutable -Destination $destinationExecutable
    $wroteDestinationExecutable = $true
    Copy-WithRetry -Source $stagedSetup -Destination $destinationSetup
    $wroteDestinationSetup = $true

    $pairs = @(
      @{ Role = 'portable'; Source = $sourceExecutable; Destination = $destinationExecutable },
      @{ Role = 'installer'; Source = $sourceSetup.FullName; Destination = $destinationSetup }
    )
    $manifestFiles = @()
    foreach ($pair in $pairs) {
      $sourceHash = Get-Sha256 -Path $pair.Source
      $destinationHash = Get-Sha256 -Path $pair.Destination
      if ($sourceHash -ne $destinationHash) { throw "发布文件校验失败：$($pair.Destination)" }
      $file = Get-Item -LiteralPath $pair.Destination
      if ($file.LastWriteTime -lt $startedAt) { throw "发布文件时间不属于本轮构建：$($pair.Destination)" }
      $manifestFiles += [ordered]@{
        role = $pair.Role
        fileName = $file.Name
        sizeBytes = $file.Length
        sha256 = $destinationHash
        sourceSha256 = $sourceHash
        modifiedAtUtc = $file.LastWriteTimeUtc.ToString('o')
      }
      [PSCustomObject]@{
        Path = $file.FullName
        Bytes = $file.Length
        Modified = $file.LastWriteTime.ToString('o')
        SHA256 = $destinationHash
      }
    }

    $manifestDocument = [ordered]@{
      schemaVersion = 1
      productName = 'iTime'
      version = $tauriVersion
      gitCommit = $gitCommit
      sourceDirty = $sourceDirty
      builtAtUtc = (Get-Date).ToUniversalTime().ToString('o')
      files = $manifestFiles
    }
    Write-Utf8NoBom -Path $stagedManifest -Content ($manifestDocument | ConvertTo-Json -Depth 6)
    Copy-WithRetry -Source $stagedManifest -Destination $releaseManifest
    $wroteReleaseManifest = $true

    $staleExecutables = @(Get-ChildItem -LiteralPath $releaseDirectory -Filter '*.exe' -File |
      Where-Object { $_.FullName -notin @($destinationExecutable, $destinationSetup) })
    for ($index = 0; $index -lt $staleExecutables.Count; $index += 1) {
      $stale = $staleExecutables[$index]
      $staleBackup = Join-Path $releaseDirectory ".stale-$transaction-$index-$($stale.Name).bak"
      Copy-WithRetry -Source $stale.FullName -Destination $staleBackup
      $staleExecutableBackups += @{
        Original = $stale.FullName
        Backup = $staleBackup
      }
      Remove-Item -LiteralPath $stale.FullName -Force
    }

    & $releaseVerifier -ReleaseDirectory $releaseDirectory
    if ($LASTEXITCODE -ne 0) { throw '发布 manifest 独立校验失败。' }
  } catch {
    if ($wroteDestinationExecutable -and (Test-Path -LiteralPath $backupExecutable)) {
      Copy-WithRetry -Source $backupExecutable -Destination $destinationExecutable
    } elseif ($wroteDestinationExecutable -and -not $hadDestinationExecutable) {
      Remove-Item -LiteralPath $destinationExecutable -Force -ErrorAction SilentlyContinue
    }
    if ($wroteDestinationSetup -and (Test-Path -LiteralPath $backupSetup)) {
      Copy-WithRetry -Source $backupSetup -Destination $destinationSetup
    } elseif ($wroteDestinationSetup -and -not $hadDestinationSetup) {
      Remove-Item -LiteralPath $destinationSetup -Force -ErrorAction SilentlyContinue
    }
    if ($wroteReleaseManifest -and (Test-Path -LiteralPath $backupManifest)) {
      Copy-WithRetry -Source $backupManifest -Destination $releaseManifest
    } elseif ($wroteReleaseManifest -and -not $hadReleaseManifest) {
      Remove-Item -LiteralPath $releaseManifest -Force -ErrorAction SilentlyContinue
    }
    foreach ($staleBackup in $staleExecutableBackups) {
      if ((Test-Path -LiteralPath $staleBackup.Backup) -and -not (Test-Path -LiteralPath $staleBackup.Original)) {
        Copy-WithRetry -Source $staleBackup.Backup -Destination $staleBackup.Original
      }
    }
    throw
  } finally {
    @($stagedExecutable, $stagedSetup, $backupExecutable, $backupSetup, $stagedManifest, $backupManifest) | ForEach-Object {
      Remove-Item -LiteralPath $_ -Force -ErrorAction SilentlyContinue
    }
    foreach ($staleBackup in $staleExecutableBackups) {
      Remove-Item -LiteralPath $staleBackup.Backup -Force -ErrorAction SilentlyContinue
    }
  }
} finally {
  Pop-Location
}
