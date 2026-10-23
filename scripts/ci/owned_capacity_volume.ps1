param(
    [Parameter(Mandatory=$true)][ValidateSet('Attach','Detach')][string]$Action,
    [Parameter(Mandatory=$true)][string]$Fixture,
    [Parameter(Mandatory=$true)][ValidateRange(128,2048)][int]$SizeMiB
)
$ErrorActionPreference = 'Stop'
$fixtureRoot = (Resolve-Path -LiteralPath $Fixture).ProviderPath.TrimEnd('\')
if ((Split-Path -Leaf $fixtureRoot) -notmatch '^rrflow-capacity-[a-zA-Z0-9_-]+$') { throw 'Not an owned capacity fixture' }
if ((Get-Item -LiteralPath $fixtureRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Fixture is a reparse point' }
$imagePath = [IO.Path]::GetFullPath((Join-Path $fixtureRoot 'capacity.vhd'))
if ([IO.Path]::GetDirectoryName($imagePath) -ne $fixtureRoot) { throw 'Image escaped owned fixture' }
if ($Action -eq 'Detach') {
    if (Test-Path -LiteralPath $imagePath) {
        $image = Get-DiskImage -ImagePath $imagePath
        if ($image.Attached) { Dismount-DiskImage -ImagePath $imagePath }
        if ((Get-DiskImage -ImagePath $imagePath).Attached) { throw 'Owned image remains attached' }
    }
    @{ detached = $true; image = $imagePath } | ConvertTo-Json -Compress
    exit 0
}
if (Test-Path -LiteralPath $imagePath) { throw 'Refusing to format an existing image' }
if ($imagePath.Contains('"') -or $imagePath.Contains("`n") -or $imagePath.Contains("`r")) { throw 'Invalid image command path' }
$diskpartFile = Join-Path $fixtureRoot 'create-owned-vhd.txt'
[IO.File]::WriteAllText($diskpartFile, "create vdisk file=`"$imagePath`" maximum=$SizeMiB type=fixed`r`nexit`r`n", [Text.Encoding]::Unicode)
$output = & "$env:SystemRoot\System32\diskpart.exe" /s $diskpartFile
if ($LASTEXITCODE -ne 0 -or !(Test-Path -LiteralPath $imagePath)) { throw 'Owned VHD creation unavailable' }
$image = Mount-DiskImage -ImagePath $imagePath -NoDriveLetter -Access ReadWrite -PassThru
$disk = $image | Get-Disk
if (@($disk).Count -ne 1 -or $disk.IsBoot -or $disk.IsSystem -or $disk.PartitionStyle -ne 'RAW') { throw 'Disk is not a new owned image' }
if ($disk.Size -gt ($SizeMiB * 1MB) -or $disk.Size -lt (($SizeMiB - 2) * 1MB)) { throw 'Owned disk capacity differs' }
# Resolve the disk again from this exact image before every destructive step.
function Require-OwnedDisk {
    $current = Get-DiskImage -ImagePath $imagePath | Get-Disk
    if (@($current).Count -ne 1 -or $current.Number -ne $disk.Number -or $current.UniqueId -ne $disk.UniqueId -or $current.IsBoot -or $current.IsSystem) { throw 'Owned image/disk identity changed' }
}
Require-OwnedDisk
Initialize-Disk -Number $disk.Number -PartitionStyle GPT -PassThru | Out-Null
Require-OwnedDisk
$partition = New-Partition -DiskNumber $disk.Number -UseMaximumSize -AssignDriveLetter
if (!$partition.DriveLetter -or $partition.DiskNumber -ne $disk.Number) { throw 'Owned partition identity differs' }
Require-OwnedDisk
$volume = $partition | Format-Volume -FileSystem NTFS -NewFileSystemLabel 'RRFlowCapacity' -Confirm:$false -Force
if ($volume.FileSystem -ne 'NTFS' -or $volume.DriveLetter -ne $partition.DriveLetter) { throw 'Owned NTFS format differs' }
@{ root = "$($partition.DriveLetter):\"; filesystem = 'ntfs'; image = $imagePath;
   disk_number = $disk.Number; disk_unique_id = $disk.UniqueId; volume_id = $volume.UniqueId;
   total_bytes = $volume.Size } | ConvertTo-Json -Compress
