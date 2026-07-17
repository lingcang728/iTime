$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'src-tauri\Cargo.toml'
$tauriConfig = Join-Path $root 'src-tauri\tauri.conf.json'
$packageJson = Join-Path $root 'package.json'
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
  & npm run verify
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

  try {
    Copy-WithRetry -Source $sourceExecutable -Destination $stagedExecutable
    Copy-WithRetry -Source $sourceSetup.FullName -Destination $stagedSetup
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $sourceExecutable).Hash -ne (Get-FileHash -Algorithm SHA256 -LiteralPath $stagedExecutable).Hash) {
      throw '可执行文件暂存校验失败。'
    }
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $sourceSetup.FullName).Hash -ne (Get-FileHash -Algorithm SHA256 -LiteralPath $stagedSetup).Hash) {
      throw '安装包暂存校验失败。'
    }

    if (Test-Path -LiteralPath $destinationExecutable) { Copy-WithRetry -Source $destinationExecutable -Destination $backupExecutable }
    if (Test-Path -LiteralPath $destinationSetup) { Copy-WithRetry -Source $destinationSetup -Destination $backupSetup }
    try {
      Copy-WithRetry -Source $stagedExecutable -Destination $destinationExecutable
      Copy-WithRetry -Source $stagedSetup -Destination $destinationSetup
    } catch {
      if (Test-Path -LiteralPath $backupExecutable) { Copy-WithRetry -Source $backupExecutable -Destination $destinationExecutable }
      if (Test-Path -LiteralPath $backupSetup) { Copy-WithRetry -Source $backupSetup -Destination $destinationSetup }
      throw
    }

    Get-ChildItem -LiteralPath $releaseDirectory -Filter '*.exe' -File |
      Where-Object { $_.FullName -notin @($destinationExecutable, $destinationSetup) } |
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
      if ($sourceHash -ne $destinationHash) { throw "发布文件校验失败：$($pair.Destination)" }
      $file = Get-Item -LiteralPath $pair.Destination
      if ($file.LastWriteTime -lt $startedAt) { throw "发布文件时间不属于本轮构建：$($pair.Destination)" }
      [PSCustomObject]@{
        Path = $file.FullName
        Bytes = $file.Length
        Modified = $file.LastWriteTime.ToString('o')
        SHA256 = $destinationHash
      }
    }
  } finally {
    @($stagedExecutable, $stagedSetup, $backupExecutable, $backupSetup) | ForEach-Object {
      Remove-Item -LiteralPath $_ -Force -ErrorAction SilentlyContinue
    }
  }
} finally {
  Pop-Location
}
