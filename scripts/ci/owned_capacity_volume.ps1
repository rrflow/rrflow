param(
    [Parameter(Mandatory=$true)][ValidateSet('Attach','Detach')][string]$Action,
    [Parameter(Mandatory=$true)][string]$Fixture,
    [Parameter(Mandatory=$true)][ValidateRange(128,2048)][int]$SizeMiB
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
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
    [Console]::Out.WriteLine(('RRFLOW_CAPACITY_VOLUME=' + (@{ detached = $true; image = $imagePath } | ConvertTo-Json -Compress)))
    [Console]::Out.Flush()
    return
}
if (Test-Path -LiteralPath $imagePath) { throw 'Refusing to format an existing image' }
if ($imagePath.Contains('"') -or $imagePath.Contains("`n") -or $imagePath.Contains("`r")) { throw 'Invalid image command path' }
# Create the exact new image through the native API. No diskpart text parser,
# shell encoding, Hyper-V installation or existing physical source is involved.
Add-Type -TypeDefinition @"
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class RRFlowOwnedVhd {
    [StructLayout(LayoutKind.Sequential)]
    struct StorageType { public uint Device; public Guid Vendor; }
    [StructLayout(LayoutKind.Sequential)]
    struct VersionOne {
        public Guid UniqueId; public ulong MaximumSize;
        public uint BlockSize; public uint SectorSize;
        public IntPtr Parent; public IntPtr Source;
    }
    [StructLayout(LayoutKind.Sequential)]
    struct Parameters { public uint Version; public VersionOne One; }
    [DllImport("virtdisk.dll", CharSet=CharSet.Unicode)]
    static extern uint CreateVirtualDisk(ref StorageType type, string path,
        uint access, IntPtr security, uint flags, uint provider,
        ref Parameters parameters, IntPtr overlapped, out IntPtr handle);
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool CloseHandle(IntPtr handle);
    public static void Create(string path, ulong size) {
        var type = new StorageType { Device = 2,
            Vendor = new Guid("ec984aec-a0f9-47e9-901f-71415a66345b") };
        var parameters = new Parameters { Version = 1,
            One = new VersionOne { MaximumSize = size, SectorSize = 512 } };
        IntPtr handle;
        uint result = CreateVirtualDisk(ref type, path, 0x003f0000,
            IntPtr.Zero, 1, 0, ref parameters, IntPtr.Zero, out handle);
        if (result != 0) throw new Win32Exception((int)result, "CreateVirtualDisk owned fixed image");
        if (!CloseHandle(handle)) throw new Win32Exception(Marshal.GetLastWin32Error());
    }
}
"@
[RRFlowOwnedVhd]::Create($imagePath, [uint64]$SizeMiB * 1MB)
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
$receipt = @{ root = "$($partition.DriveLetter):\"; filesystem = 'ntfs'; image = $imagePath;
   disk_number = $disk.Number; disk_unique_id = $disk.UniqueId; volume_id = $volume.UniqueId;
   total_bytes = $volume.Size } | ConvertTo-Json -Compress

[Console]::Out.WriteLine(('RRFLOW_CAPACITY_VOLUME=' + $receipt))
[Console]::Out.Flush()
