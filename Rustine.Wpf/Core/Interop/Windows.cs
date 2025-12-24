using System.Runtime.InteropServices;

namespace Rustine.Wpf.Core.Interop;

static unsafe partial class Windows
{
    [LibraryImport("User32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SetCursorPos(int X, int Y);

    [LibraryImport("User32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool RegisterRawInputDevices(RAWINPUTDEVICE* pRawInputDevices, uint uiNumDevices, uint cbSize);

    [LibraryImport("User32.dll")]
    public static partial uint GetRawInputData(nint hRawInput, RAW_INPUT_DATA_COMMAND_FLAGS uiCommand, [Optional] void* pData, uint* pcbSize, uint cbSizeHeader);

    public static uint GetRawInputData(nint hRawInput, RAW_INPUT_DATA_COMMAND_FLAGS uiCommand, void* pData, ref uint pcbSize, uint cbSizeHeader)
    {
        fixed (uint* pcbSizeLocal = &pcbSize)
        {
            uint __result = GetRawInputData(hRawInput, uiCommand, pData, pcbSizeLocal, cbSizeHeader);
            return __result;
        }
    }
}

struct RAWINPUTDEVICE
{
    internal ushort usUsagePage;
    internal ushort usUsage;
    internal RAWINPUTDEVICE_FLAGS dwFlags;
    internal nint hwndTarget;
}

[Flags]
enum RAWINPUTDEVICE_FLAGS : uint
{
    RIDEV_REMOVE = 0x00000001,
    RIDEV_EXCLUDE = 0x00000010,
    RIDEV_PAGEONLY = 0x00000020,
    RIDEV_NOLEGACY = 0x00000030,
    RIDEV_INPUTSINK = 0x00000100,
    RIDEV_CAPTUREMOUSE = 0x00000200,
    RIDEV_NOHOTKEYS = 0x00000200,
    RIDEV_APPKEYS = 0x00000400,
    RIDEV_EXINPUTSINK = 0x00001000,
    RIDEV_DEVNOTIFY = 0x00002000,
}

enum RAW_INPUT_DATA_COMMAND_FLAGS : uint
{
    RID_HEADER = 268435461U,
    RID_INPUT = 268435459U,
}

struct RAWINPUTHEADER
{
    internal uint dwType;
    internal uint dwSize;
    internal nint hDevice;
    internal nint wParam;
}

struct RAWMOUSE
{
    internal ushort usFlags;
    internal Union_Anonymous Anonymous;
    internal uint ulRawButtons;
    internal int lLastX;
    internal int lLastY;
    internal uint ulExtraInformation;

    [StructLayout(LayoutKind.Explicit)]
    internal partial struct Union_Anonymous
    {
        [FieldOffset(0)]
        internal uint ulButtons;
        [FieldOffset(0)]
        internal _Anonymous_e__Struct Anonymous;

        internal partial struct _Anonymous_e__Struct
        {
            internal ushort usButtonFlags;
            internal ushort usButtonData;
        }
    }
}

struct RAWKEYBOARD
{
    internal ushort MakeCode;
    internal ushort Flags;
    internal ushort Reserved;
    internal ushort VKey;
    internal uint Message;
    internal uint ExtraInformation;
}

struct RAWHID
{
    internal uint dwSizeHid;
    internal uint dwCount;
    internal byte bRawData;
}

struct RAWINPUT
{
    internal RAWINPUTHEADER header;
    internal Union_data data;

    [StructLayout(LayoutKind.Explicit)]
    internal partial struct Union_data
    {
        [FieldOffset(0)]
        internal RAWMOUSE mouse;
        [FieldOffset(0)]
        internal RAWKEYBOARD keyboard;
        [FieldOffset(0)]
        internal RAWHID hid;
    }
}