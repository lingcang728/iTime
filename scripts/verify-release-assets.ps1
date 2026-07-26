param(
  [Parameter(Mandatory = $true)][string]$ManifestPath,
  [Parameter(Mandatory = $true)][string]$AssetsDirectory,
  [switch]$RequireCleanSource
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

[PSCustomObject]@{
  Manifest = (Resolve-Path -LiteralPath $ManifestPath).Path
  AssetsDirectory = (Resolve-Path -LiteralPath $AssetsDirectory).Path
  Version = $manifest.version
  GitCommit = $manifest.gitCommit
  Files = @($expected.Values)
  Verified = $true
}
