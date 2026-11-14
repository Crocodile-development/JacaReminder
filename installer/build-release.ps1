# Builds the release exe and then the installer, in one go:
#   powershell -ExecutionPolicy Bypass -File installer\build-release.ps1
#
# The exe is built so that no path of this computer ends up inside it. Without this, Rust writes
# paths such as C:\Users\<name>\.cargo\registry\... into the exe (they appear in its error
# messages), and the installer would carry the user name of whoever built it.
#   - the home folder is written as "~" and this project as "src";
#   - the path of the debug file inside the exe is cut to the file name.
# CARGO_ENCODED_RUSTFLAGS replaces the flags of .cargo/config.toml, so the C runtime flag is
# repeated here. The version of the installer is the one in Cargo.toml.

$root = Split-Path $PSScriptRoot -Parent
$flags = @(
    '-C', 'target-feature=+crt-static',
    "--remap-path-prefix=$env:USERPROFILE=~",
    "--remap-path-prefix=$root=src",
    '-C', 'link-arg=/PDBALTPATH:%_PDB%'
)
# Flags are separated by the character 0x1f, so a folder with spaces in its name is no problem.
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]0x1f

Push-Location $root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

    $version = (Select-String -Path 'Cargo.toml' -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
    $iscc = Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'
    & $iscc "/DAppVersion=$version" 'installer\jaca-reminder.iss'
    if ($LASTEXITCODE -ne 0) { throw 'the installer could not be built' }
    Write-Host "Done: dist\JacaReminder-Setup-$version.exe"
}
finally {
    Pop-Location
}
