$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$jsonFiles = Get-ChildItem -Path (Join-Path $repositoryRoot "contracts") -Recurse -File -Filter *.json

if ($jsonFiles.Count -eq 0) {
    throw "No contract JSON files were found."
}

foreach ($jsonFile in $jsonFiles) {
    $null = Get-Content -Raw -LiteralPath $jsonFile.FullName | ConvertFrom-Json
}

$event = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot "contracts/fixtures/event.turn_started.json") | ConvertFrom-Json
if ($event.schema_version -ne 1 -or $event.kind -ne "turn.started") {
    throw "The turn-started event fixture does not match contract version 1."
}

$request = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot "contracts/fixtures/execution.request.json") | ConvertFrom-Json
if (-not $request.id -or -not $request.capability_id -or $request.ancestry.Count -gt 3) {
    throw "The execution request fixture violates required contract invariants."
}

Write-Output "Validated $($jsonFiles.Count) contract JSON files."
