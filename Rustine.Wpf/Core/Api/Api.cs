using System.Runtime.InteropServices;

namespace Rustine.Wpf.Core.Api;

enum Status : int
{
    Ok = 0,
    Error = 1,
    NotSupported = 2,
    InvalidOperation = 3,
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

/// <summary>
/// Delegate for the log callback function that receives FFI events from Rust.
/// </summary>
public delegate void FfiLogCallback(in FfiEvent ffiEvent);

static partial class Api
{
    const string LibraryName = "rustine";

    [LibraryImport(LibraryName, EntryPoint = "initialize")]
    public static partial Status Initialize(FfiLogCallback callback);

    [LibraryImport(LibraryName, EntryPoint = "terminate")]
    public static partial Status Terminate();
}