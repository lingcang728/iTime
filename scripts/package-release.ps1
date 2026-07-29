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
$latestManifest = Join-Path $releaseDirectory 'latest.json'
$releaseVerifier = Join-Path $PSScriptRoot 'verify-release-manifest.ps1'
$localSigningBackup = if ([string]::IsNullOrWhiteSpace($env:ITIME_UPDATER_BACKUP_DIR)) {
  Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'iTime-Updater-Offline-Backup'
} else {
  $env:ITIME_UPDATER_BACKUP_DIR
}
$loadedLocalSigningKey = $false

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

function Get-TextSha256 {
  param([Parameter(Mandatory = $true)][string]$Value)

  $algorithm = [System.Security.Cryptography.SHA256]::Create()
  try {
    return (($algorithm.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($Value)) |
      ForEach-Object { $_.ToString('X2') }) -join '')
  } finally {
    $algorithm.Dispose()
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
    Remove-Item -LiteralPath (Join-Path $bundleDirectory "$expectedSetupName.sig") -Force -ErrorAction SilentlyContinue
  }

  if ([string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY) -and
      [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY_PATH)) {
    $privateKeyPath = Join-Path $localSigningBackup 'itime-updater.key'
    $encryptedPasswordPath = Join-Path $localSigningBackup 'itime-updater-password.dpapi'
    if (-not (Test-Path -LiteralPath $privateKeyPath -PathType Leaf) -or
        -not (Test-Path -LiteralPath $encryptedPasswordPath -PathType Leaf)) {
      throw '缺少 Tauri updater 签名密钥；拒绝生成不可验证的更新包。'
    }
    $securePassword = ConvertTo-SecureString (Get-Content -LiteralPath $encryptedPasswordPath -Raw)
    $credential = [System.Management.Automation.PSCredential]::new('itime-updater', $securePassword)
    $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -LiteralPath $privateKeyPath -Raw).Trim()
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $credential.GetNetworkCredential().Password
    $loadedLocalSigningKey = $true
  }

  $startedAt = Get-Date
  & npm run tauri:build
  if ($LASTEXITCODE -ne 0) { throw 'Tauri 打包失败。' }

  $sourceSetup = Get-ChildItem -LiteralPath $bundleDirectory -Filter $expectedSetupName -File | Select-Object -First 1
  if (-not (Test-Path -LiteralPath $sourceExecutable)) { throw '本轮构建没有生成可直接运行的 iTime.exe。' }
  if (-not $sourceSetup) { throw "本轮构建没有生成 $expectedSetupName。" }
  $sourceSignature = "$($sourceSetup.FullName).sig"
  if (-not (Test-Path -LiteralPath $sourceSignature -PathType Leaf)) {
    throw "本轮构建没有生成 updater 签名：$sourceSignature"
  }
  $sourceExecutableFile = Get-Item -LiteralPath $sourceExecutable
  $sourceSignatureFile = Get-Item -LiteralPath $sourceSignature
  if ($sourceExecutableFile.LastWriteTime -lt $startedAt -or
      $sourceSetup.LastWriteTime -lt $startedAt -or
      $sourceSignatureFile.LastWriteTime -lt $startedAt) {
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
  $stagedLatest = Join-Path $releaseDirectory ".latest-$transaction.json.new"
  $backupLatest = Join-Path $releaseDirectory ".latest-$transaction.json.bak"
  $hadDestinationExecutable = Test-Path -LiteralPath $destinationExecutable
  $hadDestinationSetup = Test-Path -LiteralPath $destinationSetup
  $hadReleaseManifest = Test-Path -LiteralPath $releaseManifest
  $hadLatestManifest = Test-Path -LiteralPath $latestManifest
  $wroteDestinationExecutable = $false
  $wroteDestinationSetup = $false
  $wroteReleaseManifest = $false
  $wroteLatestManifest = $false
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
    if ($hadLatestManifest) { Copy-WithRetry -Source $latestManifest -Destination $backupLatest }
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
    $signature = (Get-Content -LiteralPath $sourceSignature -Raw).Trim()
    if ([string]::IsNullOrWhiteSpace($signature)) { throw 'updater 签名内容为空。' }
    $installerUrl = "https://github.com/lingcang728/iTime/releases/download/v$tauriVersion/$expectedSetupName"
    $releaseNotes = if ([string]::IsNullOrWhiteSpace($env:ITIME_RELEASE_NOTES)) {
      'Windows 本地图标自动读取、统一 AI Agent 编程工具授权与匿名性能观测、GitHub 签名自动更新。'
    } else {
      $env:ITIME_RELEASE_NOTES.Trim()
    }
    $latestDocument = [ordered]@{
      version = $tauriVersion
      notes = $releaseNotes
      pub_date = (Get-Date).ToUniversalTime().ToString('o')
      size = (Get-Item -LiteralPath $destinationSetup).Length
      platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
          signature = $signature
          url = $installerUrl
          size = (Get-Item -LiteralPath $destinationSetup).Length
        }
      }
    }
    Write-Utf8NoBom -Path $stagedLatest -Content ($latestDocument | ConvertTo-Json -Depth 6)
    Copy-WithRetry -Source $stagedLatest -Destination $latestManifest
    $wroteLatestManifest = $true
    $manifestDocument.updaterManifest = [ordered]@{
      fileName = 'latest.json'
      sha256 = Get-Sha256 -Path $latestManifest
      installerUrl = $installerUrl
      installerSignatureSha256 = Get-TextSha256 -Value $signature
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

    # Sync the current-user install used by the desktop shortcut so the user
    # always opens this build after package:release (not a stale AppData copy).
    $localDeploy = Join-Path $PSScriptRoot 'deploy-local-install.ps1'
    & $localDeploy -ReleaseExecutable $destinationExecutable
    if ($LASTEXITCODE -ne 0) { throw '本机安装目录 / 桌面快捷方式同步失败。' }
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
    if ($wroteLatestManifest -and (Test-Path -LiteralPath $backupLatest)) {
      Copy-WithRetry -Source $backupLatest -Destination $latestManifest
    } elseif ($wroteLatestManifest -and -not $hadLatestManifest) {
      Remove-Item -LiteralPath $latestManifest -Force -ErrorAction SilentlyContinue
    }
    foreach ($staleBackup in $staleExecutableBackups) {
      if ((Test-Path -LiteralPath $staleBackup.Backup) -and -not (Test-Path -LiteralPath $staleBackup.Original)) {
        Copy-WithRetry -Source $staleBackup.Backup -Destination $staleBackup.Original
      }
    }
    throw
  } finally {
    @(
      $stagedExecutable,
      $stagedSetup,
      $backupExecutable,
      $backupSetup,
      $stagedManifest,
      $backupManifest,
      $stagedLatest,
      $backupLatest
    ) | ForEach-Object {
      Remove-Item -LiteralPath $_ -Force -ErrorAction SilentlyContinue
    }
    foreach ($staleBackup in $staleExecutableBackups) {
      Remove-Item -LiteralPath $staleBackup.Backup -Force -ErrorAction SilentlyContinue
    }
  }
} finally {
  if ($loadedLocalSigningKey) {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
  }
  Pop-Location
}
