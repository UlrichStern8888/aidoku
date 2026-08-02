$ErrorActionPreference = 'Stop'

if (-not (Get-Command aidoku -ErrorAction SilentlyContinue)) {
	throw 'La commande aidoku est introuvable. Installez-la avec: cargo install --git https://github.com/Aidoku/aidoku-rs aidoku-cli'
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$packages = Join-Path $projectRoot 'packages'
New-Item -ItemType Directory -Force -Path $packages | Out-Null

Get-ChildItem -LiteralPath (Join-Path $projectRoot 'sources') -Directory | ForEach-Object {
	$sourceDirectory = $_.FullName
	$previousTargetDirectory = $env:CARGO_TARGET_DIR
	try {
		# aidoku-cli copies the first WASM it finds, so isolate each source.
		$env:CARGO_TARGET_DIR = Join-Path $sourceDirectory 'target'
		& aidoku package $sourceDirectory
		if ($LASTEXITCODE -ne 0) {
			throw "Échec de l'empaquetage de $($_.Name)"
		}
	}
	finally {
		$env:CARGO_TARGET_DIR = $previousTargetDirectory
	}
	Copy-Item -LiteralPath (Join-Path $sourceDirectory 'package.aix') `
		-Destination (Join-Path $packages "$($_.Name).aix") -Force
}

$files = (Get-ChildItem -LiteralPath $packages -Filter '*.aix').FullName
& aidoku verify $files
if ($LASTEXITCODE -ne 0) {
	throw 'La validation Aidoku a échoué.'
}

$public = Join-Path $projectRoot 'public'
& aidoku build $files --name 'Ulrichstern Aidoku Sources' --output $public
if ($LASTEXITCODE -ne 0) {
	throw 'La génération de la liste Aidoku a échoué.'
}

Write-Host "Paquets : $packages"
Write-Host "Liste Aidoku : $public\index.min.json"
