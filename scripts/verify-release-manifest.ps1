param(
  [string]$ReleaseDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'release')
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $ReleaseDirectory 'release-manifest.json'
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

[PSCustomObject]@{
  Manifest = $manifestPath
  Version = $manifest.version
  GitCommit = $manifest.gitCommit
  SourceDirty = [bool]$manifest.sourceDirty
  Files = $expectedNames
  Verified = $true
}
