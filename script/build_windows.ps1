$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo build -p family-core --release
if ($LASTEXITCODE -ne 0) { throw 'Rust core build failed' }
cargo build -p family-video-indexer --release
if ($LASTEXITCODE -ne 0) { throw 'Local video indexer build failed' }
dotnet build platforms/windows/FamilyRoom.csproj -c Release -p:Platform=x64
if ($LASTEXITCODE -ne 0) { throw 'Windows build failed' }
