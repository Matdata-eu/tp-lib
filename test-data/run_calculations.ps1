param([switch]$Verbose)
$ErrorActionPreference = "Continue"
$root = "C:\Users\mathi\Git\infrabel\tp-lib"
$cli = "$root\target\release\tp-cli.exe"
$network = "$root\test-data\network_airport.geojson"
$td = "$root\test-data"

function Run-CLI {
    param($args_list, $desc)
    Write-Host "`n==> $desc"
    $result = & $cli @args_list 2>&1
    if ($LASTEXITCODE -eq 0) { Write-Host "    OK" }
    else { Write-Host "    FAILED (exit $LASTEXITCODE)"; $result | Write-Host }
    return $LASTEXITCODE
}

# Helper to run all 3 calculations for a given log
function Process-Log {
    param($folder, $stem, [string]$extraArgs = "")

    # Determine the actual CSV path (might still be at root for locked files)
    $csv_in_folder = "$td\$folder\${stem}.csv"
    $csv_at_root   = "$td\${stem}.csv"
    $csv = if (Test-Path $csv_in_folder) { $csv_in_folder } else { $csv_at_root }

    if (-not (Test-Path $csv)) {
        Write-Host "SKIP $stem - CSV not found"
        return
    }

    $out = "$td\$folder"

    # simple-projection
    Run-CLI @("simple-projection","--gnss",$csv,"--crs","EPSG:4326","--network",$network,"--output","$out\${stem}-simple-projection.geojson") "simple-projection: $stem" | Out-Null

    # calculate-path
    Run-CLI @("calculate-path","--gnss",$csv,"--crs","EPSG:4326","--network",$network,"--output","$out\${stem}-path-calculation.geojson") "calculate-path: $stem" | Out-Null

    # extract and print segment IDs + probabilities from the path-calculation output
    $pathCalcFile = "$out\${stem}-path-calculation.geojson"
    if (Test-Path $pathCalcFile) {
        $json = Get-Content $pathCalcFile -Raw | ConvertFrom-Json
        $segs = $json.features | Where-Object {
            $_.properties.PSObject.Properties.Name -contains "netelement_id"
        }
        if (-not $segs) { $segs = $json.features }
        Write-Host "    Path segments:"
        $i = 1
        foreach ($seg in $segs) {
            $p    = $seg.properties
            $id   = if ($p.netelement_id) { $p.netelement_id } elseif ($p.id) { $p.id } else { "?" }
            $prob = if ($null -ne $p.probability) { [math]::Round($p.probability, 3) } else { "?" }
            Write-Host ("      {0,2}. {1,-12} (prob={2})" -f $i, $id, $prob)
            $i++
        }
    }

    # path projection (default, no subcommand)
    Run-CLI @("--gnss",$csv,"--crs","EPSG:4326","--network",$network,"--output","$out\${stem}-path-projection.geojson") "path-projection: $stem" | Out-Null
}

# ── Easy cases (already have outputs, re-generate to subfolders) ──────────────
Process-Log "log_28876" "log_28876_L36-B"
Process-Log "log_29083" "log_29083_L36-A"

# ── Switch cases ──────────────────────────────────────────────────────────────
Process-Log "log_28554" "log_28554_L36-A_to_L36C-A"
Process-Log "log_29304" "log_29304_L36-B_to_L36N-B"
Process-Log "log_30908" "log_30908_L36C-B_to_L36-A"
Process-Log "log_31176" "log_31176_25N-B_to_L36C-B"
Process-Log "log_31241" "log_31241_L36-B_to_L36C-B_to_L25N-A"
Process-Log "log_32870" "log_32870_L36-B_to_L36N-B"

# ── Multi-switch cases ────────────────────────────────────────────────────────
Process-Log "log_28573" "log_28573_L36-A_to_L36C-A_to_L25N-B"
Process-Log "log_29493" "log_29493_L36-A_to_L36C-A_to_L25N-B"
Process-Log "log_29584" "log_29584_L36-A_to_L36C-A_to_L25N-B"
Process-Log "log_29835" "log_29835_L36-A_to_L36C-A_to_L25N-B"
Process-Log "log_31259" "log_31259_L36-A_to_L36C-A_to_L25N-B"

# ── L36N (airport branch) ─────────────────────────────────────────────────────
Process-Log "log_29224" "log_29224_L36N-A"
Process-Log "log_30779" "log_30779_L36N-A"

# ── Very bad GNSS ─────────────────────────────────────────────────────────────
Process-Log "log_28586" "log_28586_L36-A_to_L36C-A_to_L25N-B-very-bad"
Process-Log "log_38373" "log_38373_L36-A-very-bad"

Write-Host "`n=== All done ==="
