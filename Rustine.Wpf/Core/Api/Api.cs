using System.Runtime.InteropServices;

namespace Rustine.Wpf.Core.Api;

public enum Status : int
{
    Error = -1,
    Success = 0,
    NotSupported = 2,
    InvalidOperation = 3,
}

public enum Platform : int
{
    Windows = 0,
    Wayland = 1,
    X11 = 2,
    MacOS = 3,
}

/// <summary>
/// Represents an FFI log event from Rust.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct FfiEvent
{
    public int Severity;              // 0=Debug, 1=Info, 2=Warning, 3=Error
    public ulong TimestampSecs;       // Seconds since UNIX_EPOCH
    public uint TimestampNanos;       // Nanoseconds
    public nint Message;              // Pointer to message bytes
    public nuint MessageLength;       // Length of message
    public nint Origin;               // Pointer to origin bytes
    public nuint OriginLength;        // Length of origin
    public nint Thread;               // Pointer to thread name bytes
    public nuint ThreadLength;        // Length of thread name
}

[StructLayout(LayoutKind.Sequential)]
public struct StartupParameters
{
    public nint Callback;
    public uint EnableDebugging;
    public ulong PhysicalDeviceId;
    public Platform HostPlatform;
    public Version HostVersion;
    public nint HostName;         // Pointer to UTF-8 bytes
}

[StructLayout(LayoutKind.Sequential)]
public struct PresentationParameters
{
    public uint Width;
    public uint Height;
    public nint SurfaceHandle;
    public nint SurfaceSyncHandle;
}

[StructLayout(LayoutKind.Sequential)]
public struct Version(int Major, int Minor, int Patch)
{
    public uint Major = (uint)Major;
    public uint Minor = (uint)Minor;
    public uint Patch = (uint)Patch;
}

/// <summary>
/// Delegate for the log callback function that receives FFI events from Rust.
/// </summary>
public delegate void RustineLogCallback(in FfiEvent ffiEvent);

static partial class Api
{
    const string LibraryName = "rustine";

    [LibraryImport(LibraryName, EntryPoint = "rustine_startup")]
    public static partial Status Startup(nint startupParameters);

    [LibraryImport(LibraryName, EntryPoint = "rustine_shutdown")]
    public static partial Status Shutdown();

    [LibraryImport(LibraryName, EntryPoint = "rustine_gfx_initialize_presentation")]
    public static partial Status InitializePresentation(nint presentationParameters);
}