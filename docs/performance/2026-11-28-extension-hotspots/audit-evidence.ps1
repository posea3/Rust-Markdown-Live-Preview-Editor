$ErrorActionPreference = 'Stop'
$cohorts = @('profile-before-warmed','profile-before-10mb','normal-before-warmed','normal-before-10mb','normal-after-warmed','normal-after-10mb','profile-after-warmed','profile-after-10mb','normal-before-core','normal-after-core','profile-censored')
$outerGroups = 0
$innerGroups = 0
$uniqueFixtureFiles = 0
$validatedOuterCalls = 0
$stageNames = @('validation','source_text','protected_ranges','scanner_0','candidate_filter_0','pending_sort','conflict_merge','final_sort')
function Audit-Summary($rows, $summary) {
    $groups = @($rows | Where-Object kind -eq 'sample' | Group-Object fixture,phase,stage)
    if ($groups.Count -ne $summary.Count) { throw 'Different raw/summary group counts' }
    foreach ($group in $groups) {
        $first = $group.Group[0]
        $actual = @($summary | Where-Object { $_.fixture -eq $first.fixture -and $_.phase -eq $first.phase -and $_.stage -eq $first.stage })
        $values = @($group.Group | ForEach-Object { [double]$_.ns / 1e6 } | Sort-Object)
        $middle = [int][Math]::Floor($values.Count / 2)
        $median = if ($values.Count % 2) { $values[$middle] } else { ($values[$middle - 1] + $values[$middle]) / 2 }
        if ($actual.Count -ne 1 -or [int]$actual[0].completed_samples -ne $values.Count -or [Math]::Abs([double]$actual[0].min_ms - $values[0]) -gt 1e-8 -or [Math]::Abs([double]$actual[0].median_ms - $median) -gt 1e-8 -or [Math]::Abs([double]$actual[0].max_ms - $values[-1]) -gt 1e-8) { throw ('Summary mismatch: ' + $group.Name) }
    }
    return $groups.Count
}
foreach ($cohort in $cohorts) {
    $dir = Join-Path $PSScriptRoot $cohort
    $processes = @(Import-Csv -LiteralPath (Join-Path $dir 'processes.csv'))
    $rows = @()
    foreach ($process in $processes) {
        $fixtureRows = @(Import-Csv -LiteralPath (Join-Path $dir ($process.fixture + '.csv')))
        if (@($fixtureRows | Group-Object fixture,kind,sample,phase,stage | Where-Object Count -gt 1).Count) { throw 'Duplicate outer schema2 key' }
        $uniqueFixtureFiles++
        $rows += $fixtureRows
        $log = Join-Path $dir ($process.fixture + '.stages.log')
        if ($cohort -like 'normal-*' -and (Select-String -LiteralPath $log -Pattern '^extension-')) { throw 'Normal binary emitted internal markers' }
    }
    $outerGroups += Audit-Summary $rows @(Import-Csv -LiteralPath (Join-Path $dir 'summary.csv'))
    if ($cohort -like 'profile-*') {
        $inner = @(Import-Csv -LiteralPath (Join-Path $dir 'extension-stages.csv'))
        $innerGroups += Audit-Summary $inner @(Import-Csv -LiteralPath (Join-Path $dir 'extension-summary.csv'))
        $completedKeys = @{}
        foreach ($outerCall in ($rows | Where-Object { $_.stage -eq 'scan' -and $_.kind -in @('warmup','sample') })) {
            $key = @($outerCall.fixture,$outerCall.kind,$outerCall.sample,$outerCall.phase) -join '|'
            $matching = @($inner | Where-Object { $_.fixture -eq $outerCall.fixture -and $_.kind -eq $outerCall.kind -and $_.sample -eq $outerCall.sample -and $_.phase -eq $outerCall.phase })
            if (@($matching | Group-Object call).Count -ne 1 -or ($matching.stage -join '|') -ne ($stageNames -join '|')) { throw ('Missing complete inner invocation: '+$key) }
            $completedKeys[$key] = $true
            $validatedOuterCalls++
        }
        foreach ($innerCall in ($inner | Group-Object fixture,kind,sample,phase,call)) {
            $first = $innerCall.Group[0]
            $key = @($first.fixture,$first.kind,$first.sample,$first.phase) -join '|'
            if ($cohort -ne 'profile-censored' -and -not $completedKeys.ContainsKey($key)) { throw 'Inner invocation has no matching outer call' }
        }
        if ($cohort -ne 'profile-censored') {
            foreach ($call in ($inner | Group-Object fixture,kind,sample,phase,call)) {
                if (($call.Group.stage -join '|') -ne ($stageNames -join '|')) { throw 'Missing/reordered completed stage' }
            }
        } else {
            $censored = @(Import-Csv -LiteralPath (Join-Path $dir 'extension-censored.csv'))
            if ($inner.Count -ne 4 -or ($inner.stage -join '|') -ne (($stageNames | Select-Object -First 4) -join '|') -or $censored.Count -ne 1 -or $censored[0].stage -ne 'candidate_filter_0' -or $processes[0].status -ne 'timeout') { throw 'Censored prefix incorrectly interpreted' }
        }
    }
}
$metaMatches = 0
function Compare-Metadata([string]$left, [string]$right, [string]$fixture) {
    $a = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot "$left/$fixture.csv") | Where-Object kind -like 'meta-*' | Select-Object fixture,kind,sample,phase,stage,items | ConvertTo-Csv -NoTypeInformation)
    $b = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot "$right/$fixture.csv") | Where-Object kind -like 'meta-*' | Select-Object fixture,kind,sample,phase,stage,items | ConvertTo-Csv -NoTypeInformation)
    if (Compare-Object $a $b) { throw "Metadata changed: $left/$right/$fixture" }
    return $a.Count - 1
}
$comparisonRows = @()
foreach ($pair in @(@('normal-before-warmed','normal-after-warmed'),@('normal-before-10mb','normal-after-10mb'),@('normal-before-core','normal-after-core'),@('profile-before-warmed','profile-after-warmed'),@('profile-before-10mb','profile-after-10mb'))) {
    $leftProcesses = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/processes.csv')))
    foreach ($process in $leftProcesses) { $metaMatches += Compare-Metadata $pair[0] $pair[1] $process.fixture }
    foreach ($summaryName in @('summary.csv','extension-summary.csv')) {
        if (-not (Test-Path -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/' + $summaryName)))) { continue }
        $leftSummary = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/' + $summaryName)))
        $rightSummary = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[1] + '/' + $summaryName)))
        foreach ($before in $leftSummary) {
            $after = @($rightSummary | Where-Object { $_.fixture -eq $before.fixture -and $_.phase -eq $before.phase -and $_.stage -eq $before.stage })
            if ($after.Count -ne 1 -or $before.completed_samples -ne $after[0].completed_samples) { throw 'Unpaired timing group' }
            $comparisonRows += [pscustomobject]@{ cohort_before = $pair[0]; cohort_after = $pair[1]; scope = if ($summaryName -eq 'summary.csv') { 'outer' } else { 'extension-internal' }; fixture = $before.fixture; phase = $before.phase; stage = $before.stage; before_n = $before.completed_samples; after_n = $after[0].completed_samples; before_ms = $before.median_ms; after_ms = $after[0].median_ms; before_over_after = if ([double]$after[0].median_ms -gt 0) { [Math]::Round([double]$before.median_ms / [double]$after[0].median_ms, 4) } else { $null } }
        }
    }
}
foreach ($pair in @(@('normal-before-warmed','profile-before-warmed'),@('normal-before-10mb','profile-before-10mb'),@('normal-after-warmed','profile-after-warmed'),@('normal-after-10mb','profile-after-10mb'))) {
    foreach ($process in (Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/processes.csv')))) { $metaMatches += Compare-Metadata $pair[0] $pair[1] $process.fixture }
}
$comparisonRows | Export-Csv -LiteralPath (Join-Path $PSScriptRoot 'comparison.csv') -NoTypeInformation -Encoding utf8
[pscustomobject]@{ outer_summary_groups = $outerGroups; internal_summary_groups = $innerGroups; unique_schema2_fixture_files = $uniqueFixtureFiles; validated_outer_invocations_including_warmups = $validatedOuterCalls; matching_metadata_rows = $metaMatches; timing_comparisons = $comparisonRows.Count; censored_filter_observations = 0 } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'verification.json') -Encoding utf8
Get-Content -LiteralPath (Join-Path $PSScriptRoot 'verification.json')
$comparisonRows | Where-Object { $_.scope -eq 'outer' -and $_.phase -eq 'edit' -and $_.stage -eq 'scan' -and $_.cohort_before -like 'normal-*' } | ConvertTo-Csv -NoTypeInformation
