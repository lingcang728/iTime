param(
  [Parameter(Mandatory = $true)][string]$TagName,
  [switch]$RequireTagAtHead,
  [switch]$RequireCleanSource
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$packageJsonPath = Join-Path $root 'package.json'
$packageLockPath = Join-Path $root 'package-lock.json'
$cargoManifestPath = Join-Path $root 'src-tauri\Cargo.toml'
$tauriConfigPath = Join-Path $root 'src-tauri\tauri.conf.json'

$packageVersion = (Get-Content -LiteralPath $packageJsonPath -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content -LiteralPath $tauriConfigPath -Raw | ConvertFrom-Json).version
$metadataRaw = & cargo metadata --format-version 1 --no-deps --manifest-path $cargoManifestPath
if ($LASTEXITCODE -ne 0) { throw '无法解析 Cargo 元数据。' }
$metadata = $metadataRaw | ConvertFrom-Json
$manifestPath = (Resolve-Path -LiteralPath $cargoManifestPath).Path
$cargoPackage = $metadata.packages |
  Where-Object { [System.IO.Path]::GetFullPath($_.manifest_path) -eq $manifestPath } |
  Select-Object -First 1
if (-not $cargoPackage) { throw 'Cargo 元数据中缺少 iTime 包。' }

$lockVersions = & node -e @'
const fs = require('fs');
const lock = JSON.parse(fs.readFileSync(process.argv[1], 'utf8'));
const root = lock.packages && lock.packages[''];
if (!lock.version || !root || !root.version) process.exit(2);
process.stdout.write(lock.version + '\n' + root.version);
'@ $packageLockPath
if ($LASTEXITCODE -ne 0) { throw '无法读取 package-lock.json 根版本。' }
$lockVersion = @($lockVersions)[0].Trim()
$lockRootVersion = @($lockVersions)[1].Trim()

$versions = [ordered]@{
  package = $packageVersion
  packageLock = $lockVersion
  packageLockRoot = $lockRootVersion
  cargo = $cargoPackage.version
  tauri = $tauriVersion
}
foreach ($entry in $versions.GetEnumerator()) {
  if ($entry.Value -cne $packageVersion) {
    throw "版本不一致：package=$packageVersion, $($entry.Key)=$($entry.Value)"
  }
}

$expectedTag = "v$packageVersion"
if ($TagName -cne $expectedTag) {
  throw "标签与应用版本不一致：tag=$TagName, expected=$expectedTag"
}
if ($TagName -notmatch '^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
  throw "发布标签不是受支持的 SemVer 形式：$TagName"
}

$tagCommit = (& git -C $root rev-parse --verify "refs/tags/$TagName^{commit}" 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $tagCommit) { throw "本地不存在发布标签：$TagName" }
$tagCommit = $tagCommit.Trim()
$headCommit = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or -not $headCommit) { throw '无法读取当前 Git commit。' }
if ($RequireTagAtHead -and $tagCommit -cne $headCommit) {
  throw "发布标签不指向当前提交：tag=$tagCommit, HEAD=$headCommit"
}

$dirty = [bool]((& git -C $root status --porcelain --untracked-files=normal) -join '')
if ($RequireCleanSource -and $dirty) { throw '发布必须从干净工作树执行。' }

[PSCustomObject]@{
  Tag = $TagName
  Version = $packageVersion
  Commit = $tagCommit
  Head = $headCommit
  SourceDirty = $dirty
  Verified = $true
}
