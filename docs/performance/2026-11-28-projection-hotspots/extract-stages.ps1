param([Parameter(Mandatory)][string]$Directory)
$ErrorActionPreference = 'Stop'
$rows = @()
$censored = @()
foreach ($process in (Import-Csv -LiteralPath (Join-Path $Directory 'processes.csv'))) {
    $outer = $null
    $pending = $null
    foreach ($line in (Get-Content -LiteralPath (Join-Path $Directory ($process.fixture + '.stages.log')))) {
        $parts = $line -split ','
        if ($parts[0] -eq 'begin') {
            if ($null -ne $pending) { throw 'New outer stage with incomplete inner stage' }
            $outer = $parts
        } elseif ($parts[0] -eq 'projection-begin') {
            if ($null -eq $outer -or $outer[5] -ne 'projection' -or $null -ne $pending) { throw 'Unassociated/nested inner stage' }
            $pending = $parts
        } elseif ($parts[0] -eq 'projection-end') {
            if ($null -eq $pending -or $pending[1] -ne $parts[1] -or $pending[2] -ne $parts[2]) { throw 'Mismatched inner completion' }
            $rows += [pscustomobject]@{ fixture = $outer[1]; kind = $outer[2]; sample = $outer[3]; phase = $outer[4]; call = $parts[1]; stage = $parts[2]; ns = [long]$parts[3] }
            $pending = $null
        }
    }
    if ($null -ne $pending) {
        if ($process.status -eq 'completed') { throw 'Completed process has incomplete inner stage' }
        $censored += [pscustomobject]@{ fixture = $outer[1]; kind = $outer[2]; sample = $outer[3]; phase = $outer[4]; call = $pending[1]; stage = $pending[2] }
    }
}
if (-not $rows.Count) { throw 'No completed internal projection stages' }
if (@($rows | Group-Object fixture,kind,sample,phase,stage | Where-Object Count -gt 1).Count) { throw 'Duplicate inner identity' }
$rows | Export-Csv -LiteralPath (Join-Path $Directory 'projection-stages.csv') -NoTypeInformation -Encoding utf8
$summary = foreach ($group in ($rows | Where-Object kind -eq 'sample' | Group-Object fixture,phase,stage)) {
    $values = @($group.Group | ForEach-Object { [double]$_.ns / 1e6 } | Sort-Object)
    $n = $values.Count
    $middle = [int][Math]::Floor($n / 2)
    $median = if ($n % 2) { $values[$middle] } else { ($values[$middle - 1] + $values[$middle]) / 2 }
    $first = $group.Group[0]
    [pscustomobject]@{ fixture = $first.fixture; phase = $first.phase; stage = $first.stage; completed_samples = $n; min_ms = $values[0]; median_ms = $median; max_ms = $values[-1] }
}
$summary | Sort-Object fixture,phase,stage | Export-Csv -LiteralPath (Join-Path $Directory 'projection-summary.csv') -NoTypeInformation -Encoding utf8
if ($censored.Count) { $censored | Export-Csv -LiteralPath (Join-Path $Directory 'projection-censored.csv') -NoTypeInformation -Encoding utf8 }
[pscustomobject]@{ directory = $Directory; completed_inner_calls = $rows.Count; summary_groups = @($summary).Count; censored_inner_calls = $censored.Count } | ConvertTo-Json -Compress
