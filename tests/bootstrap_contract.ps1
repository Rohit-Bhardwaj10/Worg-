$ErrorActionPreference = 'Stop'

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "Bootstrap contract failed: $Message" }
}

$root = Split-Path -Parent $PSScriptRoot
$requiredFiles = @(
    'Cargo.toml', 'rust-toolchain.toml', 'Cargo.lock', '.gitignore',
    '.github/workflows/ci.yml', 'README.md', 'plan.md', 'Decisions.md', 'changelog.md', 'AGENTS.md'
)
foreach ($relative in $requiredFiles) {
    Assert-True (Test-Path (Join-Path $root $relative) -PathType Leaf) "missing $relative"
}

$metadata = cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json
$expected = @('wm-core', 'wm-platform', 'wm-daemon', 'wmctl')
$actual = @($metadata.packages | ForEach-Object { $_.name } | Sort-Object)
Assert-True (($actual -join ',') -eq (($expected | Sort-Object) -join ',')) 'workspace must contain exactly the four planned packages'

$workflow = Get-Content (Join-Path $root '.github/workflows/ci.yml') -Raw
foreach ($command in @('cargo fmt --all -- --check', 'cargo clippy --workspace --all-targets -- -D warnings', 'cargo test --workspace --all-targets', 'cargo build --workspace --all-targets')) {
    Assert-True ($workflow.Contains($command)) "CI is missing command: $command"
}

$guide = Get-Content (Join-Path $root 'AGENTS.md') -Raw
foreach ($phrase in @('tests before', 'Do not make a git commit', 'stop')) {
    Assert-True ($guide.Contains($phrase)) "agent guide is missing required instruction: $phrase"
}

Write-Output 'Bootstrap contract passed.'
