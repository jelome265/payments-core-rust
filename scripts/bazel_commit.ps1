# Bazel-driven commit script for Windows (PowerShell).
# This script is called via 'bazelisk run //:commit -- -m "message"'

param (
    [string]$m
)

if (-not $m) {
    Write-Error "Usage: bazelisk run //:commit -- -m `"commit message`""
    exit 1
}

Write-Host "[BAZEL-COMMIT] Staging all changes in $(Get-Location)..."
git add .

Write-Host "[BAZEL-COMMIT] Committing with message: $m"
git commit -m "$m"

if ($LASTEXITCODE -eq 0) {
    Write-Host "[BAZEL-COMMIT] Success."
} else {
    Write-Host "[BAZEL-COMMIT] Commit failed (possibly no changes to commit)."
}
