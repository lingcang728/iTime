$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'src-tauri\Cargo.toml'
$tauriConfig = Join-Path $root 'src-tauri\tauri.conf.json'
$releaseDirectory = Join-Path $root 'release'

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

Push-Location $root
try {
  $metadataRaw = & cargo metadata --format-version 1 --no-deps --manifest-path $manifest
  if ($LASTEXITCODE -ne 0) { throw '无法解析 Cargo target 目录。' }
  $targetDirectory = ($metadataRaw | ConvertFrom-Json).target_directory
  $version = (Get-Content -Raw -LiteralPath $tauriConfig | ConvertFrom-Json).version
  $sourceExecutable = Join-Path $targetDirectory 'release\itime.exe'
  $bundleDirectory = Join-Path $targetDirectory 'release\bundle\nsis'

  Remove-Item -LiteralPath $sourceExecutable -Force -ErrorAction SilentlyContinue
  if (Test-Path -LiteralPath $bundleDirectory) {
    Get-ChildItem -LiteralPath $bundleDirectory -Filter "iTime_${version}_*-setup.exe" -File -ErrorAction SilentlyContinue |
      Remove-Item -Force
  }

  $startedAt = Get-Date
  & npm run tauri:build
  if ($LASTEXITCODE -ne 0) { throw 'Tauri 打包失败。' }

  $sourceSetup = Get-ChildItem -LiteralPath $bundleDirectory -Filter "iTime_${version}_*-setup.exe" -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if (-not (Test-Path -LiteralPath $sourceExecutable)) { throw '本轮构建没有生成可直接运行的 iTime.exe。' }
  if (-not $sourceSetup) { throw '本轮构建没有生成 NSIS Setup。' }

  $sourceExecutableFile = Get-Item -LiteralPath $sourceExecutable
  if ($sourceExecutableFile.LastWriteTime -lt $startedAt -or $sourceSetup.LastWriteTime -lt $startedAt) {
    throw '检测到旧发布产物，拒绝同步 release。'
  }

  New-Item -ItemType Directory -Force -Path $releaseDirectory | Out-Null
  $destinationExecutable = Join-Path $releaseDirectory 'iTime.exe'
  $destinationSetup = Join-Path $releaseDirectory $sourceSetup.Name
  Copy-WithRetry -Source $sourceExecutable -Destination $destinationExecutable
  Copy-WithRetry -Source $sourceSetup.FullName -Destination $destinationSetup

  Get-ChildItem -LiteralPath $releaseDirectory -Filter 'iTime_*-setup.exe' -File |
    Where-Object { $_.FullName -ne $destinationSetup } |
    Remove-Item -Force

  $releaseExecutables = @(Get-ChildItem -LiteralPath $releaseDirectory -Filter '*.exe' -File)
  if ($releaseExecutables.Count -ne 2) { throw 'release 必须且只能包含两个 EXE 发布文件。' }

  $pairs = @(
    @{ Source = $sourceExecutable; Destination = $destinationExecutable },
    @{ Source = $sourceSetup.FullName; Destination = $destinationSetup }
  )
  foreach ($pair in $pairs) {
    $sourceHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $pair.Source).Hash
    $destinationHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $pair.Destination).Hash
    if ($sourceHash -ne $destinationHash) { throw "发布文件复制校验失败：$($pair.Destination)" }
    $file = Get-Item -LiteralPath $pair.Destination
    [PSCustomObject]@{
      Path = $file.FullName
      Bytes = $file.Length
      Modified = $file.LastWriteTime.ToString('o')
      SHA256 = $destinationHash
    }
  }
} finally {
  Pop-Location
}
