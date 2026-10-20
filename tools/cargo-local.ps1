param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CargoArgs
)

$ff6Root = Split-Path -Parent $PSScriptRoot
$ff6Bin = Join-Path $ff6Root '.tools/rustup/toolchains/stable-x86_64-pc-windows-msvc/bin'
$ff6Cargo = Join-Path $ff6Bin 'cargo.exe'
if (-not (Test-Path -LiteralPath $ff6Cargo)) {
    throw 'Project-local Rust is absent. Install Rust normally and use cargo, or restore the local .tools toolchain.'
}
$ff6OldPath = $env:PATH
$ff6OldCargoHome = $env:CARGO_HOME
try {
    $env:PATH = "$ff6Bin;$env:PATH"
    $env:CARGO_HOME = Join-Path $ff6Root '.tools/cargo'
    & $ff6Cargo @CargoArgs
    $ff6Code = $LASTEXITCODE
} finally {
    $env:PATH = $ff6OldPath
    $env:CARGO_HOME = $ff6OldCargoHome
}
exit $ff6Code
