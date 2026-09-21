param(
  [Parameter(Mandatory = $true)][string]$ManifestPath,
  [Parameter(Mandatory = $true)][string]$AssetsDirectory,
  [switch]$RequireCleanSource,
  # After `gh release create`, confirm the *published* updater manifest
  # (releases/latest/download/latest.json) matches the local one — a silent
  # upload/CDN mismatch would break the updater for every installed client.
  [switch]$CheckLatestEndpoint
)

$ErrorActionPreference = 'Stop'

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

if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
  throw "缺少发布 manifest：$ManifestPath"
}
if (-not (Test-Path -LiteralPath $AssetsDirectory -PathType Container)) {
  throw "缺少发布资产目录：$AssetsDirectory"
}

$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
if ($manifest.schemaVersion -ne 1) { throw "不支持的 manifest schema：$($manifest.schemaVersion)" }
if ($manifest.productName -cne 'iTime') { throw "产品名不一致：$($manifest.productName)" }
if ($RequireCleanSource -and [bool]$manifest.sourceDirty) {
  throw '正式发布 manifest 标记为脏源码，拒绝验收。'
}

$expected = [ordered]@{
  portable = 'iTime.exe'
  installer = "iTime_$($manifest.version)_x64-setup.exe"
}
$assets = @(Get-ChildItem -LiteralPath $AssetsDirectory -Filter '*.exe' -File)
if ($assets.Count -ne 2) {
  throw "发布资产必须且只能包含两个 EXE，当前为 $($assets.Count) 个。"
}
$manifestFiles = @($manifest.files)
if ($manifestFiles.Count -ne 2) {
  throw "manifest 必须且只能描述两个文件，当前为 $($manifestFiles.Count) 个。"
}

foreach ($role in $expected.Keys) {
  $fileName = $expected[$role]
  $file = Get-Item -LiteralPath (Join-Path $AssetsDirectory $fileName) -ErrorAction Stop
  $entry = @($manifestFiles | Where-Object { $_.role -ceq $role -and $_.fileName -ceq $fileName })
  if ($entry.Count -ne 1) { throw "manifest 缺少唯一的 $role 条目：$fileName" }
  $hash = Get-Sha256 -Path $file.FullName
  if ([long]$entry[0].sizeBytes -ne $file.Length) { throw "文件大小校验失败：$fileName" }
  if ($entry[0].sha256 -cne $hash) { throw "SHA-256 校验失败：$fileName" }
  if ($entry[0].sourceSha256 -cne $hash) { throw "构建源 SHA-256 校验失败：$fileName" }
}

$latestPath = Join-Path $AssetsDirectory 'latest.json'
if (-not (Test-Path -LiteralPath $latestPath -PathType Leaf)) {
  throw '发布资产缺少 latest.json。'
}
$latest = Get-Content -LiteralPath $latestPath -Raw | ConvertFrom-Json
if ($latest.version -cne $manifest.version) {
  throw '发布的 latest.json 版本与 manifest 不一致。'
}
$platform = $latest.platforms.'windows-x86_64'
if (-not $platform -or [string]::IsNullOrWhiteSpace($platform.signature)) {
  throw '发布的 latest.json 缺少 Windows x64 签名。'
}
$expectedUrl = "https://github.com/lingcang728/iTime/releases/download/v$($manifest.version)/iTime_$($manifest.version)_x64-setup.exe"
if ($platform.url -cne $expectedUrl) {
  throw '发布的 latest.json 安装包 URL 不一致。'
}
$installer = Get-Item -LiteralPath (Join-Path $AssetsDirectory $expected.installer)
if ([long]$latest.size -ne $installer.Length -or [long]$platform.size -ne $installer.Length) {
  throw '发布的 latest.json 安装包大小不一致。'
}
if ($manifest.updaterManifest.sha256 -cne (Get-Sha256 -Path $latestPath)) {
  throw '发布的 latest.json SHA-256 不一致。'
}

$endpointVerified = $false
if ($CheckLatestEndpoint) {
  $endpoint = 'https://github.com/lingcang728/iTime/releases/latest/download/latest.json'
  $localLatestHash = Get-Sha256 -Path $latestPath
  $remoteBytes = $null
  # GitHub's /latest/download/ redirect can lag the asset upload by a few
  # seconds; retry briefly before declaring a mismatch.
  for ($attempt = 1; $attempt -le 6; $attempt += 1) {
    try {
      $remoteBytes = (Invoke-WebRequest -UseBasicParsing -Uri $endpoint -TimeoutSec 30 -Headers @{ 'User-Agent' = 'iTime-release-verify' }).Content
      break
    } catch {
      if ($attempt -eq 6) { throw "无法获取已发布的 latest.json：$($_.Exception.Message)" }
      Start-Sleep -Seconds 10
    }
  }
  if ($remoteBytes -is [string]) { $remoteBytes = [System.Text.Encoding]::UTF8.GetBytes($remoteBytes) }
  $stream = [System.IO.MemoryStream]::new($remoteBytes)
  try {
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
      $remoteLatestHash = (($algorithm.ComputeHash($stream) | ForEach-Object { $_.ToString('X2') }) -join '')
    } finally {
      $algorithm.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
  if ($remoteLatestHash -cne $localLatestHash) {
    # Retry once more: a stale CDN edge can serve the previous release's file.
    Start-Sleep -Seconds 15
    $retryBytes = (Invoke-WebRequest -UseBasicParsing -Uri $endpoint -TimeoutSec 30 -Headers @{ 'User-Agent' = 'iTime-release-verify' }).Content
    if ($retryBytes -is [string]) { $retryBytes = [System.Text.Encoding]::UTF8.GetBytes($retryBytes) }
    $stream = [System.IO.MemoryStream]::new($retryBytes)
    try {
      $algorithm = [System.Security.Cryptography.SHA256]::Create()
      try {
        $remoteLatestHash = (($algorithm.ComputeHash($stream) | ForEach-Object { $_.ToString('X2') }) -join '')
      } finally {
        $algorithm.Dispose()
      }
    } finally {
      $stream.Dispose()
    }
  }
  if ($remoteLatestHash -cne $localLatestHash) {
    throw "已发布 latest.json 与本地不一致：remote=$remoteLatestHash local=$localLatestHash"
  }
  $endpointVerified = $true
}

[PSCustomObject]@{
  Manifest = (Resolve-Path -LiteralPath $ManifestPath).Path
  AssetsDirectory = (Resolve-Path -LiteralPath $AssetsDirectory).Path
  Version = $manifest.version
  GitCommit = $manifest.gitCommit
  Files = @($expected.Values)
  UpdaterManifest = $latestPath
  LatestEndpointVerified = $endpointVerified
  Verified = $true
}
