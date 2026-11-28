#Requires -Version 7.0
$ErrorActionPreference = 'Stop'
$regressionRepo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$regressionRoot = Join-Path $regressionRepo ('target/projection-trace-regressions-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $regressionRoot -Force | Out-Null
$outer = 'begin,mixed-1mb,sample,0,load,projection'
$prefix = @($outer, 'projection-begin,1,validation', 'projection-end,1,validation,17', 'projection-begin,1,source_text')
$cases = @(
    @{ Name = 'torn-empty-duration'; Lines = $prefix + 'projection-end,1,source_text,'; Terminated = $true; Status = 'timeout'; Rows = 1; Censored = 'source_text' },
    @{ Name = 'torn-numeric-without-newline'; Lines = $prefix + 'projection-end,1,source_text,123'; Terminated = $false; Status = 'timeout'; Rows = 1; Censored = 'source_text' },
    @{ Name = 'censored-first-stage'; Lines = @($outer, 'projection-begin,1,validation'); Terminated = $true; Status = 'timeout'; Rows = 0; Censored = 'validation' },
    @{ Name = 'valid-zero-duration'; Lines = @($outer, 'projection-begin,1,validation', 'projection-end,1,validation,0'); Terminated = $true; Status = 'completed'; Rows = 1; Censored = $null },
    @{ Name = 'reject-negative-duration'; Lines = @($outer, 'projection-begin,1,validation', 'projection-end,1,validation,-1'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-empty-completed-duration'; Lines = @($outer, 'projection-begin,1,validation', 'projection-end,1,validation,'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-mismatched-completion'; Lines = @($outer, 'projection-begin,1,validation', 'projection-end,99,validation,12'); Terminated = $true; Status = 'timeout'; Reject = $true }
)
$failures = @()
foreach ($case in $cases) {
    $dir = Join-Path $regressionRoot $case.Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    [pscustomobject]@{ fixture = 'mixed-1mb'; status = $case.Status } | Export-Csv -LiteralPath (Join-Path $dir 'processes.csv') -NoTypeInformation -Encoding utf8
    $log = $case.Lines -join "`n"
    if ($case.Terminated) { $log += "`n" }
    [IO.File]::WriteAllText((Join-Path $dir 'mixed-1mb.stages.log'), $log, [Text.UTF8Encoding]::new($false))
    try {
        $rejected = $false
        try { & (Join-Path $PSScriptRoot 'extract-stages.ps1') -Directory $dir | Out-Null } catch { $rejected = $true; $rejection = $_.Exception.Message }
        if ($case.Reject) {
            if (-not $rejected) { throw 'Malformed completion accepted' }
        } else {
            if ($rejected) { throw ('Valid interrupted/completed transcript rejected: ' + $rejection) }
            $rows = @(Import-Csv -LiteralPath (Join-Path $dir 'projection-stages.csv'))
            $summary = @(Import-Csv -LiteralPath (Join-Path $dir 'projection-summary.csv'))
            if ($rows.Count -ne $case.Rows -or $summary.Count -ne $case.Rows) { throw 'Censored/invalid completion became an observation' }
            $censorPath = Join-Path $dir 'projection-censored.csv'
            if ($null -ne $case.Censored) {
                $censored = @(Import-Csv -LiteralPath $censorPath)
                if ($censored.Count -ne 1 -or $censored[0].stage -ne $case.Censored) { throw 'Pending censored stage was lost' }
            } elseif (Test-Path -LiteralPath $censorPath) { throw 'Valid completion marked censored' }
        }
        'PASS: ' + $case.Name
    } catch {
        $failures += $case.Name + ': ' + $_.Exception.Message
        'FAIL: ' + $failures[-1]
    }
}
if ($failures.Count) { throw ($failures -join "`n") }
'Projection trace regressions passed; evidence: ' + $regressionRoot
