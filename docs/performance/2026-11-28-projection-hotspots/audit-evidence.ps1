$ErrorActionPreference = 'Stop'
$cohorts = @('profile-before-small','profile-before-1mb','normal-before-small','normal-before-1mb','normal-after-small','normal-after-1mb','profile-after-small','profile-after-1mb','normal-before-core','normal-after-core','normal-after-10mb','profile-censored','normal-after-1mb-warmed')
$outerGroups = 0
$innerGroups = 0
$uniqueFixtureFiles = 0
$stageNames = @('validation','source_text','reveal_groups','conceal_spans','structural_padding','builtin_candidates','extension_candidates','normalize_ownership','shadow_filter','overlap_check','map','materialize','project_blocks','project_styles','project_widgets')
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
        if ($cohort -like 'normal-*' -and (Select-String -LiteralPath $log -Pattern '^projection-')) { throw 'Normal binary emitted internal markers' }
    }
    $outerGroups += Audit-Summary $rows @(Import-Csv -LiteralPath (Join-Path $dir 'summary.csv'))
    if ($cohort -like 'profile-*') {
        $inner = @(Import-Csv -LiteralPath (Join-Path $dir 'projection-stages.csv'))
        $innerGroups += Audit-Summary $inner @(Import-Csv -LiteralPath (Join-Path $dir 'projection-summary.csv'))
        if ($cohort -ne 'profile-censored') {
            foreach ($call in ($inner | Group-Object fixture,kind,sample,phase,call)) {
                if (($call.Group.stage -join '|') -ne ($stageNames -join '|')) { throw 'Missing/reordered completed stage' }
            }
        } else {
            $censored = @(Import-Csv -LiteralPath (Join-Path $dir 'projection-censored.csv'))
            if ($inner.Count -ne 2 -or $censored.Count -ne 1 -or $censored[0].stage -ne 'reveal_groups' -or $processes[0].status -ne 'timeout') { throw 'Censored prefix incorrectly interpreted' }
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
foreach ($pair in @(@('normal-before-small','normal-after-small'),@('normal-before-1mb','normal-after-1mb'),@('normal-before-core','normal-after-core'),@('profile-before-small','profile-after-small'),@('profile-before-1mb','profile-after-1mb'))) {
    $leftProcesses = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/processes.csv')))
    foreach ($process in $leftProcesses) { $metaMatches += Compare-Metadata $pair[0] $pair[1] $process.fixture }
    foreach ($summaryName in @('summary.csv','projection-summary.csv')) {
        if (-not (Test-Path -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/' + $summaryName)))) { continue }
        $leftSummary = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/' + $summaryName)))
        $rightSummary = @(Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[1] + '/' + $summaryName)))
        foreach ($before in $leftSummary) {
            $after = @($rightSummary | Where-Object { $_.fixture -eq $before.fixture -and $_.phase -eq $before.phase -and $_.stage -eq $before.stage })
            if ($after.Count -ne 1 -or $before.completed_samples -ne $after[0].completed_samples) { throw 'Unpaired timing group' }
            $comparisonRows += [pscustomobject]@{ cohort_before = $pair[0]; cohort_after = $pair[1]; scope = if ($summaryName -eq 'summary.csv') { 'outer' } else { 'projection-internal' }; fixture = $before.fixture; phase = $before.phase; stage = $before.stage; before_n = $before.completed_samples; after_n = $after[0].completed_samples; before_ms = $before.median_ms; after_ms = $after[0].median_ms; before_over_after = [Math]::Round([double]$before.median_ms / [double]$after[0].median_ms, 4) }
        }
    }
}
foreach ($pair in @(@('normal-before-small','profile-before-small'),@('normal-before-1mb','profile-before-1mb'),@('normal-after-small','profile-after-small'),@('normal-after-1mb','profile-after-1mb'))) {
    foreach ($process in (Import-Csv -LiteralPath (Join-Path $PSScriptRoot ($pair[0] + '/processes.csv')))) { $metaMatches += Compare-Metadata $pair[0] $pair[1] $process.fixture }
}
$comparisonRows | Export-Csv -LiteralPath (Join-Path $PSScriptRoot 'comparison.csv') -NoTypeInformation -Encoding utf8
[pscustomobject]@{ outer_summary_groups = $outerGroups; internal_summary_groups = $innerGroups; unique_schema2_fixture_files = $uniqueFixtureFiles; matching_metadata_rows = $metaMatches; timing_comparisons = $comparisonRows.Count; censored_reveal_groups_observations = 0 } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'verification.json') -Encoding utf8
Get-Content -LiteralPath (Join-Path $PSScriptRoot 'verification.json')
$comparisonRows | Where-Object { $_.scope -eq 'outer' -and $_.phase -eq 'edit' -and $_.stage -eq 'projection' -and $_.cohort_before -like 'normal-*' } | ConvertTo-Csv -NoTypeInformation
