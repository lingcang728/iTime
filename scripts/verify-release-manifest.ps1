param(
  [string]$ReleaseDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'release'),
  [switch]$RequireCleanSource
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $ReleaseDirectory 'release-manifest.json'
$latestPath = Join-Path $ReleaseDirectory 'latest.json'
$packageJsonPath = Join-Path $root 'package.json'

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

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
  throw "缺少发布 manifest：$manifestPath"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schemaVersion -ne 1) { throw "不支持的 manifest schema：$($manifest.schemaVersion)" }
if ($manifest.productName -cne 'iTime') { throw "产品名不一致：$($manifest.productName)" }
if (-not $manifest.version) { throw 'manifest 缺少版本。' }
if ($RequireCleanSource -and [bool]$manifest.sourceDirty) {
  throw '正式发布 manifest 标记为脏源码，拒绝验收。'
}
if (-not $manifest.gitCommit -or $manifest.gitCommit -notmatch '^[0-9a-f]{40}$') {
  throw 'manifest 的 gitCommit 不是完整的 40 位提交。'
}

$builtAt = [DateTimeOffset]::MinValue
if (-not [DateTimeOffset]::TryParse($manifest.builtAtUtc, [ref]$builtAt)) {
  throw 'manifest 的 builtAtUtc 无法解析。'
}

$packageVersion = (Get-Content -LiteralPath $packageJsonPath -Raw | ConvertFrom-Json).version
if ($manifest.version -cne $packageVersion) {
  throw "manifest 与 package.json 版本不一致：manifest=$($manifest.version), package=$packageVersion"
}

$headCommit = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $manifest.gitCommit -cne $headCommit) {
  throw "manifest 与当前 Git commit 不一致：manifest=$($manifest.gitCommit), HEAD=$headCommit"
}

$expectedNames = @(
  'iTime.exe',
  "iTime_$($manifest.version)_x64-setup.exe"
)
$releaseExecutables = @(Get-ChildItem -LiteralPath $ReleaseDirectory -Filter '*.exe' -File)
$actualNames = @($releaseExecutables | ForEach-Object Name)
if ($actualNames.Count -ne 2) {
  throw "release 必须且只能包含两个 EXE，当前为 $($actualNames.Count) 个。"
}
foreach ($expectedName in $expectedNames) {
  if ($actualNames -cnotcontains $expectedName) { throw "release 缺少预期文件：$expectedName" }
}

$manifestFiles = @($manifest.files)
if ($manifestFiles.Count -ne 2) { throw "manifest 必须且只能描述两个文件，当前为 $($manifestFiles.Count) 个。" }
if (@($manifestFiles | ForEach-Object fileName | Select-Object -Unique).Count -ne 2) {
  throw 'manifest 文件名存在重复。'
}

foreach ($expectedName in $expectedNames) {
  $entry = @($manifestFiles | Where-Object { $_.fileName -ceq $expectedName })
  if ($entry.Count -ne 1) { throw "manifest 必须且只能包含一个条目：$expectedName" }
  $file = Get-Item -LiteralPath (Join-Path $ReleaseDirectory $expectedName)
  $hash = Get-Sha256 -Path $file.FullName
  if ([long]$entry[0].sizeBytes -ne $file.Length) { throw "文件大小校验失败：$expectedName" }
  if ($entry[0].sha256 -cne $hash) { throw "SHA-256 校验失败：$expectedName" }
  if ($entry[0].sourceSha256 -cne $hash) { throw "构建源 SHA-256 校验失败：$expectedName" }
}

$expectedRoles = [ordered]@{
  portable = 'iTime.exe'
  installer = "iTime_$($manifest.version)_x64-setup.exe"
}
foreach ($role in $expectedRoles.Keys) {
  $entry = @($manifestFiles | Where-Object {
    $_.role -ceq $role -and $_.fileName -ceq $expectedRoles[$role]
  })
  if ($entry.Count -ne 1) { throw "manifest 缺少唯一的 $role 角色映射。" }
}

if (-not (Test-Path -LiteralPath $latestPath -PathType Leaf)) {
  throw "release 缺少 updater 清单：$latestPath"
}
$latest = Get-Content -LiteralPath $latestPath -Raw | ConvertFrom-Json
if ($latest.version -cne $manifest.version) {
  throw "latest.json 与发布版本不一致：latest=$($latest.version), manifest=$($manifest.version)"
}
$platform = $latest.platforms.'windows-x86_64'
if (-not $platform -or [string]::IsNullOrWhiteSpace($platform.signature)) {
  throw 'latest.json 缺少 Windows x64 updater 签名。'
}
$expectedUrl = "https://github.com/lingcang728/iTime/releases/download/v$($manifest.version)/iTime_$($manifest.version)_x64-setup.exe"
if ($platform.url -cne $expectedUrl) {
  throw "latest.json 安装包 URL 不一致：$($platform.url)"
}
$installer = Get-Item -LiteralPath (Join-Path $ReleaseDirectory "iTime_$($manifest.version)_x64-setup.exe")
if ([long]$platform.size -ne $installer.Length -or [long]$latest.size -ne $installer.Length) {
  throw 'latest.json 安装包大小与真实文件不一致。'
}
if (-not $manifest.updaterManifest -or
    $manifest.updaterManifest.fileName -cne 'latest.json' -or
    $manifest.updaterManifest.sha256 -cne (Get-Sha256 -Path $latestPath)) {
  throw 'release manifest 与 latest.json 的摘要不一致。'
}

[PSCustomObject]@{
  Manifest = $manifestPath
  Version = $manifest.version
  GitCommit = $manifest.gitCommit
  SourceDirty = [bool]$manifest.sourceDirty
  Files = $expectedNames
  UpdaterManifest = $latestPath
  Verified = $true
}
