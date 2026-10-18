param(
    [Parameter(Mandatory = $true)][string]$Workspace,
    [switch]$RunReference
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $RunReference) { throw 'Reference execution requires explicit -RunReference invocation.' }
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'The diagnostic requires native Windows.' }
if (-not [Environment]::UserInteractive) { throw 'No interactive desktop is available; no reference process was started.' }
if (Get-Process -Name Mp3tag -ErrorAction SilentlyContinue) { throw 'An existing Mp3tag process prevents an isolated diagnostic.' }

$root = [IO.Path]::GetFullPath($Workspace).TrimEnd('\')
$temporary = @([IO.Path]::GetTempPath().TrimEnd('\'))
if ($env:GITHUB_ACTIONS -eq 'true' -and $env:RUNNER_TEMP) { $temporary += [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\') }
if (-not ($temporary | Where-Object { $root.StartsWith($_ + '\', [StringComparison]::OrdinalIgnoreCase) })) {
    throw 'Workspace must belong to an OS/runner temporary directory.'
}
function Owned-Path([string]$Relative) {
    $path = [IO.Path]::GetFullPath([IO.Path]::Combine($root, $Relative))
    if (-not $path.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Path escapes workspace.' }
    $item = $path
    while ($item.Length -ge $root.Length) {
        if ((Test-Path -LiteralPath $item) -and ((Get-Item -LiteralPath $item).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'Reparse points are not permitted in the disposable runtime.'
        }
        $item = [IO.Path]::GetDirectoryName($item)
        if (-not $item) { break }
    }
    return $path
}
function Write-Record([string]$Path, $Value) {
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 16), [Text.UTF8Encoding]::new($false))
}
$runtime = Get-Content -LiteralPath (Owned-Path 'runtime.json') -Raw | ConvertFrom-Json
if ($runtime.schema -ne 'taggroove.mp3tag-reference/v1' -or $runtime.status -ne 'unexecuted') { throw 'Unsupported runtime preparation record.' }
$exe = Owned-Path 'runtime/application/Mp3tag.exe'
$package = Owned-Path 'download/setup.exe'
if ((Get-Item -LiteralPath $exe).Length -ne 25073528 -or (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant() -ne '484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911') {
    throw 'Reference executable identity mismatch; nothing was executed.'
}
if ((Get-Item -LiteralPath $package).Length -ne 6008656 -or (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant() -ne 'a977861003a87b1f00f2e0c91263c61bf12d457c6d457b367ce5f69de28fbea0') {
    throw 'Official installer identity mismatch; nothing was executed.'
}
$cfg = Owned-Path 'runtime/application/mp3tag.cfg'
if ((Get-Item -LiteralPath $cfg).Length -ne 0) { throw 'The diagnostic requires its untouched empty configuration candidate.' }
$output = Owned-Path 'evidence/windows-probe.json'
if (Test-Path -LiteralPath $output) { throw 'A diagnostic must not overwrite prior observations.' }

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class TagGrooveReferenceWindows {
    public delegate bool Enumerate(IntPtr h, IntPtr value);
    [DllImport("user32.dll")] public static extern bool EnumWindows(Enumerate callback, IntPtr value);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, Enumerate callback, IntPtr value);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder name, int max);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder text, int max);
    [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
}
'@
$native = [Collections.Generic.List[object]]::new()
$automation = [Collections.Generic.List[object]]::new()
$errors = [Collections.Generic.List[string]]::new()
$started = $null
$creationTicks = $null
function Confirm-OwnedProcess {
    if ($null -eq $started -or $started.HasExited) { return $null }
    $live = Get-Process -Id $started.Id -ErrorAction Stop
    if ($live.StartTime.ToUniversalTime().Ticks -ne $creationTicks -or $live.MainModule.FileName -ne $exe) {
        throw 'Process creation/image identity changed; refuse observation or shutdown.'
    }
    return $live
}
$record = [ordered]@{
    schema = 'taggroove.mp3tag-reference/v1'
    observation_type = 'runtime_probe'
    status = 'probe_only'
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    runtime = @{ os = 'Windows'; os_version = [Environment]::OSVersion.VersionString; process_id = $null; session_id = $null; interactive_desktop = [Environment]::UserInteractive; collector_source_sha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant() }
    reference = @{ application = 'Mp3tag'; version = '3.36.1'; executable_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant(); installer_sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant(); installer_url = 'https://download.mp3tag.de/mp3tag-v3.36.1-x64-setup.exe' }
    configuration = @{ candidate = 'runtime/application/mp3tag.cfg'; mode = 'unverified'; path_observed = $false; bytes_after_exit = $null; sha256_after_exit = $null }
    defaults = $null
    shutdown = @{ requested = $false; clean_exit = $null; forced_termination = $false; exit_code = $null }
    save_actions_performed = $false
    native_windows = $native
    automation_elements = $automation
    errors = $errors
}
try {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $exe
    $info.UseShellExecute = $false
    $info.WorkingDirectory = [IO.Path]::GetDirectoryName($exe)
    $info.Arguments = '/fp:"' + (Owned-Path 'empty') + '"'
    # Apply overrides only to this child. Do not change global environment or registry.
    foreach ($name in @('APPDATA', 'LOCALAPPDATA', 'TEMP', 'TMP')) {
        $relative = $runtime.environment.$name
        $info.EnvironmentVariables[$name] = Owned-Path $relative
    }
    $started = [Diagnostics.Process]::Start($info)
    $record.runtime.process_id = $started.Id
    $record.runtime.session_id = $started.SessionId
    $creationTicks = $started.StartTime.ToUniversalTime().Ticks
    $record.runtime.process_creation_time_utc = $started.StartTime.ToUniversalTime().ToString('o')
    if (-not $started.WaitForInputIdle(15000)) { throw 'Reference process did not become input idle.' }
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $started.Refresh()
        if ($started.HasExited) { throw 'Reference process exited before a window was observed.' }
        if ($started.MainWindowHandle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($started.MainWindowHandle -eq [IntPtr]::Zero) { throw 'No reference window was observed in this session.' }
    [void](Confirm-OwnedProcess)
    if ((Get-FileHash -LiteralPath $started.MainModule.FileName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $record.reference.executable_sha256) {
        throw 'Observed process image does not match the pinned executable.'
    }
    $processId = $started.Id
    $inspect = [TagGrooveReferenceWindows+Enumerate]{
        param([IntPtr]$handle, [IntPtr]$unused)
        [uint32]$owner = 0
        [void][TagGrooveReferenceWindows]::GetWindowThreadProcessId($handle, [ref]$owner)
        if ($owner -ne $processId) { return $true }
        if ($native.Count -ge 512) { return $false }
        $name = [Text.StringBuilder]::new(1024)
        $text = [Text.StringBuilder]::new(4096)
        [void][TagGrooveReferenceWindows]::GetClassName($handle, $name, $name.Capacity)
        [void][TagGrooveReferenceWindows]::GetWindowText($handle, $text, $text.Capacity)
        $native.Add(@{ handle = $handle.ToInt64(); process_id = $owner; class = $name.ToString(); text = $text.ToString(); control_id = [TagGrooveReferenceWindows]::GetDlgCtrlID($handle); visible = [TagGrooveReferenceWindows]::IsWindowVisible($handle) })
        return $true
    }
    $top = [TagGrooveReferenceWindows+Enumerate]{
        param([IntPtr]$handle, [IntPtr]$unused)
        [uint32]$owner = 0
        [void][TagGrooveReferenceWindows]::GetWindowThreadProcessId($handle, [ref]$owner)
        if ($owner -eq $processId) {
            [void]$inspect.Invoke($handle, [IntPtr]::Zero)
            [void][TagGrooveReferenceWindows]::EnumChildWindows($handle, $inspect, [IntPtr]::Zero)
        }
        return $true
    }
    [void][TagGrooveReferenceWindows]::EnumWindows($top, [IntPtr]::Zero)
    $element = [System.Windows.Automation.AutomationElement]::FromHandle($started.MainWindowHandle)
    $queue = [Collections.Generic.Queue[object]]::new()
    $queue.Enqueue(@{ element = $element; depth = 0 })
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    while ($queue.Count -and $automation.Count -lt 512) {
        $next = $queue.Dequeue()
        $current = $next.element
        try {
            if ($current.Current.ProcessId -ne $processId) { continue }
            $item = @{ name = $current.Current.Name; class = $current.Current.ClassName; automation_id = $current.Current.AutomationId; control_type = $current.Current.ControlType.ProgrammaticName; process_id = $current.Current.ProcessId; depth = $next.depth; patterns = @($current.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }) }
            $pattern = $null
            if ($current.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) { $item.toggle_state = $pattern.Current.ToggleState.ToString() }
            $automation.Add($item)
            if ($next.depth -lt 8) {
                $child = $walker.GetFirstChild($current)
                while ($null -ne $child -and ($queue.Count + $automation.Count) -lt 512) { $queue.Enqueue(@{ element = $child; depth = $next.depth + 1 }); $child = $walker.GetNextSibling($child) }
            }
        } catch { $errors.Add($_.Exception.Message) }
    }
} catch {
    $record.status = 'blocked'
    $errors.Add($_.Exception.Message)
} finally {
    if ($null -ne $started) {
        try {
            $live = Confirm-OwnedProcess
            if ($null -ne $live) {
                $record.shutdown.requested = $true
                [void]$live.CloseMainWindow()
                $record.shutdown.clean_exit = $started.WaitForExit(5000)
                if (-not $record.shutdown.clean_exit) {
                    $live = Confirm-OwnedProcess
                    if ($null -ne $live) {
                        $record.shutdown.forced_termination = $true
                        $live.Kill()
                        if (-not $started.WaitForExit(5000)) { throw 'Owned process did not terminate within the shutdown bound.' }
                    }
                    $errors.Add('Owned process required termination; clean configuration shutdown was not observed.')
                }
            }
            if ($started.HasExited) { $record.shutdown.exit_code = $started.ExitCode }
        } catch { $record.status = 'blocked'; $errors.Add($_.Exception.Message) }
        $cfgFile = Get-Item -LiteralPath $cfg
        $record.configuration.bytes_after_exit = $cfgFile.Length
        $record.configuration.sha256_after_exit = (Get-FileHash -LiteralPath $cfg -Algorithm SHA256).Hash.ToLowerInvariant()
        $record.configuration.path_observed = $cfgFile.Length -gt 0
    }
    Write-Record $output $record
}
if ($record.status -eq 'blocked') { throw "Diagnostic blocked; inspect $output. No tagging was performed." }
Write-Output "Diagnostic only: $output. Portable mode, selected defaults and save/undo/redo are not yet qualified."
