#pragma warning disable CS0649

using System.Runtime.InteropServices;

using static Rustine.Wpf.Core.Utilities.Exceptions;

namespace Rustine.Wpf.Core.Interop;

enum D3D_DRIVER_TYPE : uint
{
    UNKNOWN = 0,
    HARDWARE,
    REFERENCE,
    NULL,
    SOFTWARE,
    WARP
}

[Flags]
enum D3D11_CREATE_DEVICE_FLAG : uint
{
    NONE = 0,
    SINGLETHREADED = 0x1,
    DEBUG = 0x2,
    SWITCH_TO_REF = 0x4,
    PREVENT_INTERNAL_THREADING_OPTIMIZATIONS = 0x8,
    BGRA_SUPPORT = 0x20,
    DEBUGGABLE = 0x40,
    PREVENT_ALTERING_LAYER_SETTINGS_FROM_REGISTRY = 0x80,
    DISABLE_GPU_TIMEOUT = 0x100,
    VIDEO_SUPPORT = 0x800
}

enum D3DDEVTYPE : uint
{
    NONE = 0,
    HAL = 1,
    NULLREF = 4,
    REF = 2,
    SW = 3,
}

[Flags]
enum D3DCREATE : uint
{
    FPU_PRESERVE = 0x00000002,
    MULTITHREADED = 0x00000004,
    PUREDEVICE = 0x00000010,
    SOFTWARE_VERTEXPROCESSING = 0x00000020,
    HARDWARE_VERTEXPROCESSING = 0x00000040,
    MIXED_VERTEXPROCESSING = 0x00000080,
    DISABLE_DRIVER_MANAGEMENT = 0x00000100,
    ADAPTERGROUP_DEVICE = 0x00000200,
    DISABLE_DRIVER_MANAGEMENT_EX = 0x00000400,
    NOWINDOWCHANGES = 0x00000800,
    DISABLE_PSGP_THREADING = 0x00002000,
    ENABLE_PRESENTSTATS = 0x00004000,
    DISABLE_PRINTSCREEN = 0x00008000,
    SCREENSAVER = 0x10000000,
}

static unsafe partial class DirectX
{
    [LibraryImport("d3d9.dll")]
    public static partial uint Direct3DCreate9Ex(uint SDKVersion, IDirect3D9Ex** direct3D9Ex);

    public static void CreateD3D9Ex(uint SDKVersion, out IDirect3D9Ex direct3D9Ex)
    {
        fixed (IDirect3D9Ex* direct3D9Ex_ = &direct3D9Ex)
        {
            ThrowIfNonZero(Direct3DCreate9Ex(SDKVersion, (IDirect3D9Ex**)direct3D9Ex_));
        }
    }

    [LibraryImport("d3d11.dll")]
    public static partial uint D3D11CreateDevice(
        nint pAdapter,
        uint DriverType,
        nint Software,
        uint Flags,
        nint pFeatureLevels,
        uint FeatureLevels,
        uint SDKVersion,
        ID3D11Device** ppDevice,
        nint pFeatureLevel,
        ID3D11DeviceContext** ppImmediateContext);

    public static void CreateD3D11Device(
        nint pAdapter,
        D3D_DRIVER_TYPE DriverType,
        nint Software,
        D3D11_CREATE_DEVICE_FLAG Flags,
        nint pFeatureLevels,
        uint FeatureLevels,
        uint SDKVersion,
        out ID3D11Device ppDevice,
        out ID3D11DeviceContext ppImmediateContext)
    {
        fixed (ID3D11Device* ppDevice_ = &ppDevice)
        fixed (ID3D11DeviceContext* ppImmediateContext_ = &ppImmediateContext)
        {
            ThrowIfNonZero(D3D11CreateDevice(
                pAdapter,
                (uint)DriverType,
                Software,
                (uint)Flags,
                pFeatureLevels,
                FeatureLevels,
                SDKVersion,
                (ID3D11Device**)ppDevice_,
                0,
                (ID3D11DeviceContext**)ppImmediateContext_));
        }
    }

    public static unsafe void SafeRelease<T>(ref T* pInterface) where T : unmanaged
    {
        if (pInterface == null)
            return;

        IUnknown* pInterfaceAsIUnknown = (IUnknown*)pInterface;
        if (pInterfaceAsIUnknown != null)
        {
            pInterfaceAsIUnknown->Release();
            pInterface = null;
        }
    }
}

struct D3DPRESENT_PARAMETERS(uint backBufferWidth, uint backBufferHeight, uint backBufferFormat, uint backBufferCount, uint multiSampleType, uint multiSampleQuality, uint swapEffect, nint hDeviceWindow, uint windowed, uint enableAutoDepthStencil, uint autoDepthStencilFormat, uint flags, uint fullScreen_RefreshRateInHz, uint presentationInterval)
{
    public uint BackBufferWidth = backBufferWidth;
    public uint BackBufferHeight = backBufferHeight;
    public uint BackBufferFormat = backBufferFormat;
    public uint BackBufferCount = backBufferCount;
    public uint MultiSampleType = multiSampleType;
    public uint MultiSampleQuality = multiSampleQuality;
    public uint SwapEffect = swapEffect;
    public nint hDeviceWindow = hDeviceWindow;
    public uint Windowed = windowed;
    public uint EnableAutoDepthStencil = enableAutoDepthStencil;
    public uint AutoDepthStencilFormat = autoDepthStencilFormat;
    public uint Flags = flags;
    public uint FullScreen_RefreshRateInHz = fullScreen_RefreshRateInHz;
    public uint PresentationInterval = presentationInterval;
}

internal unsafe struct IUnknown
{
    struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<IUnknown*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IUnknown*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IUnknown*, uint> Release;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IUnknown*)lpVtbl);
            lpVtbl = null;
        }
    }

    public uint QueryInterface<TOut>(TOut** ppvObject) where TOut : unmanaged
    {
        Guid riid = Marshal.GenerateGuidForType(typeof(TOut));
        return (*lpVtbl)->QueryInterface((IUnknown*)lpVtbl, &riid, (void**)ppvObject);
    }
}

internal unsafe struct IDirect3D9Ex
{
    struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, uint> Release;

        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, nint, void> PLACEHOLDER_RegisterSoftwareDevice;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, uint> PLACEHOLDER_GetAdapterCount;

        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_GetAdapterIdentifier;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_GetAdapterModeCount;

        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_EnumAdapterModes;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_GetAdapterDisplayMode;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_CheckDeviceType;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_CheckDeviceFormat;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_CheckDeviceMultiSampleType;

        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_CheckDepthStencilMatch;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_CheckDeviceFormatConversion;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_GetDeviceCaps;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_GetAdapterMonitor;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_CreateDevice;

        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_GetAdapterModeCountEx;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_EnumAdapterModesEx;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, void> PLACEHOLDER_GetAdapterDisplayModeEx;
        internal delegate* unmanaged[Stdcall]<IDirect3D9Ex*, uint, uint, nint, uint, void*, nint, void**, uint> CreateDeviceEx;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDirect3D9Ex*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void CreateDeviceEx(uint adapter, D3DDEVTYPE deviceType, nint windowHandle, D3DCREATE behaviorFlags, ref D3DPRESENT_PARAMETERS presentParameters, out IDirect3DDevice9Ex pDeviceEx)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDirect3D9Ex));

        fixed (D3DPRESENT_PARAMETERS* pPresentParameters = &presentParameters)
        fixed (IDirect3DDevice9Ex* ppDeviceEx = &pDeviceEx)
            ThrowIfNonZero((*lpVtbl)->CreateDeviceEx((IDirect3D9Ex*)lpVtbl, adapter, (uint)deviceType, windowHandle, (uint)behaviorFlags, pPresentParameters, 0, (void**)ppDeviceEx));
    }
}

enum D3DUSAGE : uint
{
    NONE = 0x00000000,
    RENDERTARGET = 0x00000001,
    DEPTHSTENCIL = 0x00000002,
    DYNAMIC = 0x00000200,
    NONSECURE = 0x00800000,
    AUTOGENMIPMAP = 0x00000400,
    DMAP = 0x00004000,
    QUERY_LEGACYBUMPMAP = 0x00008000,
    QUERY_SRGBREAD = 0x00010000,
    QUERY_FILTER = 0x00020000,
    QUERY_SRGBWRITE = 0x00040000,
    QUERY_POSTPIXELSHADER_BLENDING = 0x00080000,
    QUERY_VERTEXTEXTURE = 0x00100000,
    QUERY_WRAPANDMIP = 0x00200000,
    WRITEONLY = 0x00000008,
    SOFTWAREPROCESSING = 0x00000010,
    DONOTCLIP = 0x00000020,
    POINTS = 0x00000040,
    RTPATCHES = 0x00000080,
    NPATCHES = 0x00000100,
    TEXTAPI = 0x10000000,
    RESTRICTED_CONTENT = 0x00000800,
    RESTRICT_SHARED_RESOURCE = 0x00002000,
    RESTRICT_SHARED_RESOURCE_DRIVER = 0x00001000,
}

enum D3DFORMAT : uint
{
    UNKNOWN = 0,
    R8G8B8 = 20,
    A8R8G8B8 = 21,
    X8R8G8B8 = 22,
    R5G6B5 = 23,
    X1R5G5B5 = 24,
    A1R5G5B5 = 25,
    A4R4G4B4 = 26,
    R3G3B2 = 27,
    A8 = 28,
    A8R3G3B2 = 29,
    X4R4G4B4 = 30,
    A2B10G10R10 = 31,
    A8B8G8R8 = 32,
    X8B8G8R8 = 33,
    G16R16 = 34,
    A2R10G10B10 = 35,
    A16B16G16R16 = 36,
    A8P8 = 40,
    P8 = 41,
    L8 = 50,
    A8L8 = 51,
    A4L4 = 52,
    V8U8 = 60,
    L6V5U5 = 61,
    X8L8V8U8 = 62,
    Q8W8V8U8 = 63,
    V16U16 = 64,
    A2W10V10U10 = 67,
    //UYVY = MAKEFOURCC('U', 'Y', 'V', 'Y'),
    //R8G8_B8G8 = MAKEFOURCC('R', 'G', 'B', 'G'),
    //YUY2 = MAKEFOURCC('Y', 'U', 'Y', '2'),
    //G8R8_G8B8 = MAKEFOURCC('G', 'R', 'G', 'B'),
    //DXT1 = MAKEFOURCC('D', 'X', 'T', '1'),
    //DXT2 = MAKEFOURCC('D', 'X', 'T', '2'),
    //DXT3 = MAKEFOURCC('D', 'X', 'T', '3'),
    //DXT4 = MAKEFOURCC('D', 'X', 'T', '4'),
    //DXT5 = MAKEFOURCC('D', 'X', 'T', '5'),
    D16_LOCKABLE = 70,
    D32 = 71,
    D15S1 = 73,
    D24S8 = 75,
    D24X8 = 77,
    D24X4S4 = 79,
    D16 = 80,
    D32F_LOCKABLE = 82,
    D24FS8 = 83,
    D32_LOCKABLE = 84,
    S8_LOCKABLE = 85,
    L16 = 81,
    VERTEXDATA = 100,
    INDEX16 = 101,
    INDEX32 = 102,
    Q16W16V16U16 = 110,
    //MULTI2_ARGB8 = MAKEFOURCC('M', 'E', 'T', '1'),
    R16F = 111,
    G16R16F = 112,
    A16B16G16R16F = 113,
    R32F = 114,
    G32R32F = 115,
    A32B32G32R32F = 116,
    CxV8U8 = 117,
    A1 = 118,
    A2B10G10R10_XR_BIAS = 119,
    BINARYBUFFER = 199,
}

internal unsafe struct IDirect3DDevice9Ex
{
    struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, uint> Release;

        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_TestCooperativeLevel;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetAvailableTextureMem;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_EvictManagedResources;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetDirect3D;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetDeviceCaps;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetDisplayMode;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetCreationParameters;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_SetCursorProperties;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_SetCursorPosition;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_ShowCursor;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_CreateAdditionalSwapChain;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetSwapChain;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetNumberOfSwapChains;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_Reset;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_Present;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetBackBuffer;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetRasterStatus;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_SetDialogBoxMode;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_SetGammaRamp;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, void> PLACEHOLDER_GetGammaRamp;
        internal delegate* unmanaged[Stdcall]<IDirect3DDevice9Ex*, uint, uint, uint, uint, uint, uint, void**, void**, uint> CreateTexture;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDirect3DDevice9Ex*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void CreateTexture(
        uint width,
        uint height,
        uint levels,
        D3DUSAGE usage,
        D3DFORMAT format,
        uint pool,
        out IDirect3DTexture9 pTexture,
        out nint pSharedHandle)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDirect3DDevice9Ex));

        fixed (void* ppSharedHandle = &pSharedHandle)
        fixed (IDirect3DTexture9* ppTexture = &pTexture)
            ThrowIfNonZero((*lpVtbl)->CreateTexture((IDirect3DDevice9Ex*)lpVtbl, width, height, levels, (uint)usage, (uint)format, pool, (void**)ppTexture, (void**)ppSharedHandle));
    }
}

internal unsafe struct IDirect3DTexture9
{
    struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, uint> Release;

        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetDevice;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_SetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_FreePrivateData;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_SetPriority;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetPriority;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_PreLoad;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetType;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_SetLOD;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetLOD;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetLevelCount;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_SetAutoGenFilterType;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetAutoGenFilterType;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GenerateMipSubLevels;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, void> PLACEHOLDER_GetLevelDesc;
        internal delegate* unmanaged[Stdcall]<IDirect3DTexture9*, uint, void**, uint> GetSurfaceLevel;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDirect3DTexture9*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void GetSurfaceLevel(uint level, out IDirect3DSurface9 ppSurface)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDirect3DTexture9));

        fixed (IDirect3DSurface9* ppSurfaceLevel_ = &ppSurface)
            ThrowIfNonZero((*lpVtbl)->GetSurfaceLevel((IDirect3DTexture9*)lpVtbl, level, (void**)ppSurfaceLevel_));
    }
}

internal unsafe struct IDirect3DSurface9
{
    struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<IDirect3DSurface9*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDirect3DSurface9*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDirect3DSurface9*, uint> Release;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public readonly nint Get()
    {
        return (nint)(IDirect3DSurface9*)lpVtbl;
    }

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDirect3DSurface9*)lpVtbl);
            lpVtbl = null;
        }
    }
}

enum DXGI_FORMAT : uint
{
    UNKNOWN = 0,
    R32G32B32A32_TYPELESS = 1,
    R32G32B32A32_FLOAT = 2,
    R32G32B32A32_UINT = 3,
    R32G32B32A32_SINT = 4,
    R32G32B32_TYPELESS = 5,
    R32G32B32_FLOAT = 6,
    R32G32B32_UINT = 7,
    R32G32B32_SINT = 8,
    R16G16B16A16_TYPELESS = 9,
    R16G16B16A16_FLOAT = 10,
    R16G16B16A16_UNORM = 11,
    R16G16B16A16_UINT = 12,
    R16G16B16A16_SNORM = 13,
    R16G16B16A16_SINT = 14,
    R32G32_TYPELESS = 15,
    R32G32_FLOAT = 16,
    R32G32_UINT = 17,
    R32G32_SINT = 18,
    R32G8X24_TYPELESS = 19,
    D32_FLOAT_S8X24_UINT = 20,
    R32_FLOAT_X8X24_TYPELESS = 21,
    X32_TYPELESS_G8X24_UINT = 22,
    R10G10B10A2_TYPELESS = 23,
    R10G10B10A2_UNORM = 24,
    R10G10B10A2_UINT = 25,
    R11G11B10_FLOAT = 26,
    R8G8B8A8_TYPELESS = 27,
    R8G8B8A8_UNORM = 28,
    R8G8B8A8_UNORM_SRGB = 29,
    R8G8B8A8_UINT = 30,
    R8G8B8A8_SNORM = 31,
    R8G8B8A8_SINT = 32,
    R16G16_TYPELESS = 33,
    R16G16_FLOAT = 34,
    R16G16_UNORM = 35,
    R16G16_UINT = 36,
    R16G16_SNORM = 37,
    R16G16_SINT = 38,
    R32_TYPELESS = 39,
    D32_FLOAT = 40,
    R32_FLOAT = 41,
    R32_UINT = 42,
    R32_SINT = 43,
    R24G8_TYPELESS = 44,
    D24_UNORM_S8_UINT = 45,
    R24_UNORM_X8_TYPELESS = 46,
    X24_TYPELESS_G8_UINT = 47,
    R8G8_TYPELESS = 48,
    R8G8_UNORM = 49,
    R8G8_UINT = 50,
    R8G8_SNORM = 51,
    R8G8_SINT = 52,
    R16_TYPELESS = 53,
    R16_FLOAT = 54,
    D16_UNORM = 55,
    R16_UNORM = 56,
    R16_UINT = 57,
    R16_SNORM = 58,
    R16_SINT = 59,
    R8_TYPELESS = 60,
    R8_UNORM = 61,
    R8_UINT = 62,
    R8_SNORM = 63,
    R8_SINT = 64,
    A8_UNORM = 65,
    R1_UNORM = 66,
    R9G9B9E5_SHAREDEXP = 67,
    R8G8_B8G8_UNORM = 68,
    G8R8_G8B8_UNORM = 69,
    BC1_TYPELESS = 70,
    BC1_UNORM = 71,
    BC1_UNORM_SRGB = 72,
    BC2_TYPELESS = 73,
    BC2_UNORM = 74,
    BC2_UNORM_SRGB = 75,
    BC3_TYPELESS = 76,
    BC3_UNORM = 77,
    BC3_UNORM_SRGB = 78,
    BC4_TYPELESS = 79,
    BC4_UNORM = 80,
    BC4_SNORM = 81,
    BC5_TYPELESS = 82,
    BC5_UNORM = 83,
    BC5_SNORM = 84,
    B5G6R5_UNORM = 85,
    B5G5R5A1_UNORM = 86,
    B8G8R8A8_UNORM = 87,
    B8G8R8X8_UNORM = 88,
    R10G10B10_XR_BIAS_A2_UNORM = 89,
    B8G8R8A8_TYPELESS = 90,
    B8G8R8A8_UNORM_SRGB = 91,
    B8G8R8X8_TYPELESS = 92,
    B8G8R8X8_UNORM_SRGB = 93,
    BC6H_TYPELESS = 94,
    BC6H_UF16 = 95,
    BC6H_SF16 = 96,
    BC7_TYPELESS = 97,
    BC7_UNORM = 98,
    BC7_UNORM_SRGB = 99,
    AYUV = 100,
    Y410 = 101,
    Y416 = 102,
    NV12 = 103,
    P010 = 104,
    P016 = 105,
    _420_OPAQUE = 106,
    YUY2 = 107,
    Y210 = 108,
    Y216 = 109,
    NV11 = 110,
    AI44 = 111,
    IA44 = 112,
    P8 = 113,
    A8P8 = 114,
    B4G4R4A4_UNORM = 115,
    P208 = 130,
    V208 = 131,
    V408 = 132,
    SAMPLER_FEEDBACK_MIN_MIP_OPAQUE = 189,
    SAMPLER_FEEDBACK_MIP_REGION_USED_OPAQUE = 190,
    A4B4G4R4_UNORM = 191,
}

enum D3D11_USAGE
{
    DEFAULT = 0,
    IMMUTABLE = 1,
    DYNAMIC = 2,
    STAGING = 3
}

[Flags]
enum D3D11_BIND_FLAG : uint
{
    VERTEX_BUFFER = 0x1,
    INDEX_BUFFER = 0x2,
    CONSTANT_BUFFER = 0x4,
    SHADER_RESOURCE = 0x8,
    STREAM_OUTPUT = 0x10,
    RENDER_TARGET = 0x20,
    DEPTH_STENCIL = 0x40,
    UNORDERED_ACCESS = 0x80,
    DECODER = 0x200,
    VIDEO_ENCODER = 0x400
}

[Flags]
enum D3D11_CPU_ACCESS_FLAG : uint
{
    NONE = 0,
    WRITE = 0x10000,
    READ = 0x20000
}

[Flags]
enum D3D11_RESOURCE_MISC_FLAG : uint
{
    NONE = 0,
    GENERATE_MIPS = 0x1,
    SHARED = 0x2,
    TEXTURECUBE = 0x4,
    DRAWINDIRECT_ARGS = 0x10,
    BUFFER_ALLOW_RAW_VIEWS = 0x20,
    BUFFER_STRUCTURED = 0x40,
    RESOURCE_CLAMP = 0x80,
    SHARED_KEYEDMUTEX = 0x100,
    GDI_COMPATIBLE = 0x200,
    SHARED_NTHANDLE = 0x800,
    RESTRICTED_CONTENT = 0x1000,
    RESTRICT_SHARED_RESOURCE = 0x2000,
    RESTRICT_SHARED_RESOURCE_DRIVER = 0x4000,
    GUARDED = 0x8000,
    TILE_POOL = 0x20000,
    TILED = 0x40000,
    HW_PROTECTED = 0x80000,
    SHARED_DISPLAYABLE,
    SHARED_EXCLUSIVE_WRITER,
    D3D11_RESOURCE_MISC_NO_SHADER_ACCESS
}

struct DXGI_SAMPLE_DESC
{
    public uint Count;
    public uint Quality;
}

struct D3D11_TEXTURE2D_DESC
{
    public uint Width;
    public uint Height;
    public uint MipLevels;
    public uint ArraySize;
    public DXGI_FORMAT Format;
    public DXGI_SAMPLE_DESC SampleDesc;
    public D3D11_USAGE Usage;
    public D3D11_BIND_FLAG BindFlags;
    public D3D11_CPU_ACCESS_FLAG CPUAccessFlags;
    public D3D11_RESOURCE_MISC_FLAG MiscFlags;
}

struct D3D11_SUBRESOURCE_DATA
{
    public nint pSysMem;
    public uint SysMemPitch;
    public uint SysMemSlicePitch;
}

struct D3D11_MAPPED_SUBRESOURCE
{
    public nint pData;
    public uint RowPitch;
    public uint DepthPitch;
}

internal unsafe struct ID3D11Device
{
    internal struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, uint> Release;

        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateBuffer_4;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateTexture1D_5;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, D3D11_TEXTURE2D_DESC*, D3D11_SUBRESOURCE_DATA*, ID3D11Texture2D**, uint> CreateTexture2D;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateTexture3D_7;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateShaderResourceView_8;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateUnorderedAccessView_9;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateRenderTargetView_10;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateDepthStencilView_11;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateInputLayout_12;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, nuint, void> CreateVertexShader_13;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, nuint, void> CreateGeometryShader_14;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, nuint, void> CreateGeometryShaderWithStreamOutput_15;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, nuint, void> CreatePixelShader_16;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, nuint, void> CreateHullShader_17;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, nuint, void> CreateDomainShader_18;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, nuint, void> CreateComputeShader_19;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateClassLinkage_20;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateBlendState_21;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateDepthStencilState_22;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateRasterizerState_23;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateSamplerState_24;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateQuery_25;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreatePredicate_26;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateCounter_27;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CreateDeferredContext_28;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void*, Guid*, void**, uint> OpenSharedResource_29;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CheckFormatSupport_30;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CheckMultisampleQualityLevels_31;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CheckCounterInfo_32;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CheckCounter_33;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> CheckFeatureSupport_34;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> GetPrivateData_35;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> SetPrivateData_36;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> SetPrivateDataInterface_37;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> GetFeatureLevel_38;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, uint> GetCreationFlags_39;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> GetDeviceRemovedReason_40;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> GetImmediateContext_41;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> SetExceptionMode_42;
        internal delegate* unmanaged[Stdcall]<ID3D11Device*, void> GetExceptionMode_43;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((ID3D11Device*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void QueryInterface<T>(out T ppvObject) where T : unmanaged
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11Device));

        Guid riid = Marshal.GenerateGuidForType(typeof(T));

        fixed (T* ppvObject_ = &ppvObject)
            ThrowIfNonZero((*lpVtbl)->QueryInterface((ID3D11Device*)lpVtbl, &riid, (void**)ppvObject_));
    }

    public void OpenSharedResource<T>(nint hResource, out T ppResource) where T : unmanaged
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11Device));

        Guid riid = Marshal.GenerateGuidForType(typeof(T));

        fixed (T* ppResource_ = &ppResource)
            ThrowIfNonZero((*lpVtbl)->OpenSharedResource_29((ID3D11Device*)lpVtbl, (void*)hResource, &riid, (void**)ppResource_));
    }

    public void CreateTexture2D(D3D11_TEXTURE2D_DESC pDesc, D3D11_SUBRESOURCE_DATA? pInitialData, out ID3D11Texture2D ppTexture2D)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11Device));

        D3D11_SUBRESOURCE_DATA initialDataStruct;
        D3D11_SUBRESOURCE_DATA* pInitialData_ = null;
        if (pInitialData.HasValue)
        {
            initialDataStruct = pInitialData.Value;
            pInitialData_ = &initialDataStruct;
        }

        fixed (ID3D11Texture2D* ppTexture2D_ = &ppTexture2D)
            ThrowIfNonZero((*lpVtbl)->CreateTexture2D((ID3D11Device*)lpVtbl, &pDesc, pInitialData_, (ID3D11Texture2D**)ppTexture2D_));
    }
}

[Guid("c0bfa96c-e089-44fb-8eaf-26f8796190da")]
internal unsafe struct ID3D11DeviceContext
{
    internal struct Vtbl
    {
        // IUnknown
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, uint> Release;

        // ID3D11DeviceChild
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GetDevice;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GetPrivateData;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> SetPrivateData;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> SetPrivateDataInterface;

        // ID3D11DeviceContext
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSSetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSSetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSSetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSSetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSSetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DrawIndexed;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> Draw;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> Map;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> Unmap;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSSetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IASetInputLayout;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IASetVertexBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IASetIndexBuffer;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DrawIndexedInstanced;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DrawInstanced;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSSetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSSetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IASetPrimitiveTopology;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSSetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSSetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void*, void> Begin;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void*, void> End;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GetData;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> SetPredication;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSSetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSSetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMSetRenderTargets;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMSetRenderTargetsAndUnorderedAccessViews;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMSetBlendState;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMSetDepthStencilState;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> SOSetTargets;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DrawAuto;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DrawIndexedInstancedIndirect;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DrawInstancedIndirect;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> Dispatch;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DispatchIndirect;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> RSSetState;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> RSSetViewports;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> RSSetScissorRects;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CopySubresourceRegion;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void*, void*, void> CopyResource;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> UpdateSubresource;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CopyStructureCount;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> ClearRenderTargetView;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> ClearUnorderedAccessViewUint;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> ClearUnorderedAccessViewFloat;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> ClearDepthStencilView;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GenerateMips;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> SetResourceMinLOD;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GetResourceMinLOD;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> ResolveSubresource;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> ExecuteCommandList;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSSetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSSetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSSetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSSetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSSetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSSetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSSetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSSetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSSetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSSetUnorderedAccessViews;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSSetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSSetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSSetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSGetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSGetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSGetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSGetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSGetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> PSGetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IAGetInputLayout;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IAGetVertexBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IAGetIndexBuffer;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSGetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSGetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> IAGetPrimitiveTopology;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSGetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> VSGetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GetPredication;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSGetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GSGetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMGetRenderTargets;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMGetRenderTargetsAndUnorderedAccessViews;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMGetBlendState;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> OMGetDepthStencilState;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> SOGetTargets;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> RSGetState;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> RSGetViewports;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> RSGetScissorRects;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSGetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSGetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSGetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> HSGetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSGetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSGetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSGetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> DSGetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSGetShaderResources;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSGetUnorderedAccessViews;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSGetShader;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSGetSamplers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> CSGetConstantBuffers;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> ClearState;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> Flush;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GetType_;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> GetContextFlags;
        internal delegate* unmanaged[Stdcall]<ID3D11DeviceContext*, void> FinishCommandList;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((ID3D11DeviceContext*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void CopyResource(nint dst, nint src)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11DeviceContext));

        (*lpVtbl)->CopyResource((ID3D11DeviceContext*)lpVtbl, (void*)dst, (void*)src);
    }

    public void Begin()
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11DeviceContext));

        (*lpVtbl)->Begin((ID3D11DeviceContext*)lpVtbl, null);
    }

    public void End()
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11DeviceContext));

        (*lpVtbl)->End((ID3D11DeviceContext*)lpVtbl, null);
    }

    public void Flush()
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11DeviceContext));

        (*lpVtbl)->Flush((ID3D11DeviceContext*)lpVtbl);
    }
}

enum D3D11_MESSAGE_SEVERITY
{
    D3D11_MESSAGE_SEVERITY_CORRUPTION = 0,
    D3D11_MESSAGE_SEVERITY_ERROR = D3D11_MESSAGE_SEVERITY_CORRUPTION + 1,
    D3D11_MESSAGE_SEVERITY_WARNING = D3D11_MESSAGE_SEVERITY_ERROR + 1,
    D3D11_MESSAGE_SEVERITY_INFO = D3D11_MESSAGE_SEVERITY_WARNING + 1,
    D3D11_MESSAGE_SEVERITY_MESSAGE = D3D11_MESSAGE_SEVERITY_INFO + 1
}

struct D3D11_MESSAGE
{
    public uint Category;
    public D3D11_MESSAGE_SEVERITY Severity;
    public uint ID;
    public nint pDescription;
    public ulong DescriptionByteLength;
}

[Guid("6543dbb6-1b48-42f5-ab82-e97ec74326f6")]
internal unsafe struct ID3D11InfoQueue
{
    struct Vtbl
    {
        // IUnknown
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, uint> Release;

        // ID3D11InfoQueue
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> SetMessageCountLimit;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> ClearStoredMessages;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, ulong, D3D11_MESSAGE*, ulong*, uint> GetMessage;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetNumStoredMessagesAllowedByRetrievalFilters;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetNumMessagesDeniedByStorageFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, ulong> GetNumStoredMessages;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetNumStoredMessagesAllowedByRetrievalFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetNumMessagesDiscardedByMessageCountLimit;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetMessageCountLimit;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> AddStorageFilterEntries;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetStorageFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> ClearStorageFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PushEmptyStorageFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PushCopyOfStorageFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PushStorageFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PopStorageFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetStorageFilterStackSize;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> AddRetrievalFilterEntries;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetRetrievalFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> ClearRetrievalFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PushEmptyRetrievalFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PushCopyOfRetrievalFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PushRetrievalFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> PopRetrievalFilter;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetRetrievalFilterStackSize;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> AddMessage;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> AddApplicationMessage;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> SetBreakOnCategory;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> SetBreakOnSeverity;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> SetBreakOnID;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetBreakOnCategory;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetBreakOnSeverity;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetBreakOnID;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> SetMuteDebugOutput;
        internal delegate* unmanaged[Stdcall]<ID3D11InfoQueue*, void> GetMuteDebugOutput;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public readonly nint Get()
    {
        return (nint)(ID3D11InfoQueue*)lpVtbl;
    }

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((ID3D11InfoQueue*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void ClearStoredMessages()
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11InfoQueue));

        (*lpVtbl)->ClearStoredMessages((ID3D11InfoQueue*)lpVtbl);
    }

    public ulong GetNumStoredMessages()
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11InfoQueue));

        return (*lpVtbl)->GetNumStoredMessages((ID3D11InfoQueue*)lpVtbl);
    }

    public string GetMessage(ulong messageIndex)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11InfoQueue));

        ulong messageLength = 0;
        uint res = (*lpVtbl)->GetMessage((ID3D11InfoQueue*)lpVtbl, messageIndex, null, &messageLength);

        D3D11_MESSAGE* message = (D3D11_MESSAGE*)Marshal.AllocHGlobal((nint)messageLength);
        try
        {
            ThrowIfNonZero((*lpVtbl)->GetMessage((ID3D11InfoQueue*)lpVtbl, messageIndex, message, &messageLength));

            return Marshal.PtrToStringAnsi(message->pDescription) ?? string.Empty;
        }
        finally
        {
            Marshal.FreeHGlobal((nint)message);
        }
    }
}

[Flags]
enum D3D11_RLDO_FLAGS : uint
{
    SUMMARY = 0x1,
    DETAIL = 0x2,
    IGNORE_INTERNAL = 0x4
}

[Guid("79cf2233-7536-4948-9d36-1e4692dc5760")]
internal unsafe struct ID3D11Debug
{
    struct Vtbl
    {
        // IUnknown
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, uint> Release;

        // ID3D11Debug
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> SetFeatureMask;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> GetFeatureMask;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> SetPresentPerRenderOpDelay;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> GetPresentPerRenderOpDelay;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> SetSwapChain;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> GetSwapChain;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> ValidateContext;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, D3D11_RLDO_FLAGS, uint> ReportLiveDeviceObjects;
        internal delegate* unmanaged[Stdcall]<ID3D11Debug*, void> ValidateContextForDispatch;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public readonly nint Get()
    {
        return (nint)(ID3D11Debug*)lpVtbl;
    }

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((ID3D11Debug*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void QueryInterface<T>(out T ppvObject) where T : unmanaged
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11Debug));

        Guid riid = Marshal.GenerateGuidForType(typeof(T));
        fixed (T* ppvObject_ = &ppvObject)
            ThrowIfNonZero((*lpVtbl)->QueryInterface((ID3D11Debug*)lpVtbl, &riid, (void**)ppvObject_));
    }

    public void ReportLiveDeviceObjects(D3D11_RLDO_FLAGS flags)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11Debug));

        ThrowIfNonZero((*lpVtbl)->ReportLiveDeviceObjects((ID3D11Debug*)lpVtbl, flags));
    }
}

[Guid("6f15aaf2-d208-4e89-9ab4-489535d34f9c")]
internal unsafe struct ID3D11Texture2D
{
    struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<ID3D11Texture2D*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<ID3D11Texture2D*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<ID3D11Texture2D*, uint> Release;
    }

    Vtbl** lpVtbl;

    public readonly bool IsValid => lpVtbl != null;

    public readonly nint Get()
    {
        return (nint)(ID3D11Texture2D*)lpVtbl;
    }

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((ID3D11Texture2D*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void QueryInterface<T>(out T ppvObject) where T : unmanaged
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(ID3D11Texture2D));

        Guid riid = Marshal.GenerateGuidForType(typeof(T));
        fixed (T* ppvObject_ = &ppvObject)
            ThrowIfNonZero((*lpVtbl)->QueryInterface((ID3D11Texture2D*)lpVtbl, &riid, (void**)ppvObject_));
    }
}

[Guid("035f3ab4-482e-4e50-b41f-8a7f8bd8960b")]
internal unsafe struct IDXGIResource
{
    struct Vtbl
    {
        // IUnknown
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> Release;

        // IDXGIObject
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> SetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> SetPrivateDataInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> GetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> GetParent;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> GetDevice;

        // IDXGIResource
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, void**, uint> GetSharedHandle;
    }

    Vtbl** lpVtbl;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDXGIResource*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void GetSharedHandle(out nint sharedHandle)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDXGIResource));

        fixed (nint* sharedHandle_ = &sharedHandle)
            ThrowIfNonZero((*lpVtbl)->GetSharedHandle((IDXGIResource*)lpVtbl, (void**)sharedHandle_));
    }
}

enum DXGI_SHARED_RESOURCE_FLAGS : uint
{
    READ = 0x80000000,
    WRITE = 0x40000000
}

[Guid("30961379-4609-4a41-998e-54fe567ee0c1")]
internal unsafe struct IDXGIResource1
{
    struct Vtbl
    {
        // IUnknown
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> Release;

        // IDXGIObject
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> SetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> SetPrivateDataInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> GetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> GetParent;

        // IDXGIDeviceSubObject
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> GetDevice;

        // IDXGIResource
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, void**, uint> GetSharedHandle;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> GetUsage;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> SetEvictionPriority;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, uint> GetEvictionPriority;

        // IDXGIResource1
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, void> CreateSubresourceSurface;
        internal delegate* unmanaged[Stdcall]<IDXGIResource1*, void*, uint, void*, void**, uint> CreateSharedHandle;
    }

    Vtbl** lpVtbl;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDXGIResource1*)lpVtbl);
            lpVtbl = null;
        }
    }

    public void GetSharedHandle(out nint sharedHandle)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDXGIResource1));

        fixed (nint* sharedHandle_ = &sharedHandle)
            ThrowIfNonZero((*lpVtbl)->GetSharedHandle((IDXGIResource1*)lpVtbl, (void**)sharedHandle_));
    }

    public void CreateSharedHandle(DXGI_SHARED_RESOURCE_FLAGS flags, out nint sharedHandle)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDXGIResource1));

        fixed (nint* sharedHandle_ = &sharedHandle)
            ThrowIfNonZero((*lpVtbl)->CreateSharedHandle((IDXGIResource1*)lpVtbl, null, (uint)flags, null, (void**)sharedHandle_));
    }
}

[Guid("54ec77fa-1377-44e6-8c32-88fd5f44c84c")]
internal unsafe struct IDXGIDevice
{
    struct Vtbl
    {
        // IUnknown
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, uint> Release;

        // IDXGIObject 
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, uint> SetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, uint> SetPrivateDataInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, uint> GetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, uint> GetParent;

        // IDXGIDevice
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, IDXGIAdapter**, uint> GetAdapter;
        internal delegate* unmanaged[Stdcall]<IDXGIDevice*, uint, uint, uint, uint, uint, uint, void**, uint> CreateSurface;
    }

    Vtbl** lpVtbl;

    public uint QueryInterface<T>(out T* ppvObject) where T : unmanaged
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDXGIDevice));

        Guid riid = Marshal.GenerateGuidForType(typeof(T));
        fixed (T** ppvObject_ = &ppvObject)
            return (*lpVtbl)->QueryInterface((IDXGIDevice*)lpVtbl, &riid, (void**)ppvObject_);
    }

    public void GetAdapter(out IDXGIAdapter ppAdapter)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDXGIDevice));

        fixed (IDXGIAdapter* ppAdapter_ = &ppAdapter)
            ThrowIfNonZero((*lpVtbl)->GetAdapter((IDXGIDevice*)lpVtbl, (IDXGIAdapter**)ppAdapter_));
    }

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDXGIDevice*)lpVtbl);
            lpVtbl = null;
        }
    }
}

[Guid("2411e7e1-12ac-4ccf-bd14-9798e8534dc0")]
internal unsafe struct IDXGIAdapter
{
    struct Vtbl
    {
        // IUnknown
        internal delegate* unmanaged[Stdcall]<IDXGIAdapter*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIAdapter*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDXGIAdapter*, uint> Release;

        // IDXGIObject 
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> SetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> SetPrivateDataInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> GetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> GetParent;

        // IDXGIAdapter
        internal delegate* unmanaged[Stdcall]<IDXGIResource*, uint> EnumOutputs;
        internal delegate* unmanaged[Stdcall]<IDXGIAdapter*, DXGI_ADAPTER_DESC*, uint> GetDesc;
    }

    Vtbl** lpVtbl;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDXGIAdapter*)lpVtbl);
            lpVtbl = null;
        }
    }

    public uint QueryInterface<T>(out T ppvObject) where T : unmanaged
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDXGIAdapter));

        Guid riid = Marshal.GenerateGuidForType(typeof(T));
        fixed (T* ppvObject_ = &ppvObject)
            return (*lpVtbl)->QueryInterface((IDXGIAdapter*)lpVtbl, &riid, (void**)ppvObject_);
    }

    public void GetDesc(out DXGI_ADAPTER_DESC desc)
    {
        ObjectDisposedException.ThrowIf(lpVtbl == null, typeof(IDXGIAdapter));

        fixed (DXGI_ADAPTER_DESC* desc_ = &desc)
            ThrowIfNonZero((*lpVtbl)->GetDesc((IDXGIAdapter*)lpVtbl, desc_));
    }
}

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
internal unsafe struct DXGI_ADAPTER_DESC
{
    public fixed char Description[128];
    public uint VendorId;
    public uint DeviceId;
    public uint SubSysId;
    public uint Revision;
    public nint DedicatedVideoMemory;
    public nint DedicatedSystemMemory;
    public nint SharedSystemMemory;
    public LUID AdapterLuid;
}

[StructLayout(LayoutKind.Sequential)]
internal struct LUID
{
    public uint LowPart;
    public int HighPart;
}

[Guid("9d8e1289-d7b3-465f-8126-250e349af85d")]
internal unsafe struct IDXGIKeyedMutex
{
    struct Vtbl
    {
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, Guid*, void**, uint> QueryInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, uint> AddRef;
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, uint> Release;

        // IDXGIObject
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, uint> SetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, uint> SetPrivateDataInterface;
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, uint> GetPrivateData;
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, uint> GetParent;

        // IDXGIDeviceSubObject
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, uint> GetDevice;

        // IDXGIKeyedMutex
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, ulong, uint, uint> AcquireSync;
        internal delegate* unmanaged[Stdcall]<IDXGIKeyedMutex*, ulong, uint> ReleaseSync;
    }

    Vtbl** lpVtbl;

    public void Release()
    {
        if (lpVtbl != null)
        {
            (*lpVtbl)->Release((IDXGIKeyedMutex*)lpVtbl);
            lpVtbl = null;
        }
    }

    /// <summary>
    /// Using a key, acquires exclusive rendering access to a shared resource.
    /// </summary>
    /// <param name="key">A value that indicates which device to give access to. This method will succeed when the device that currently owns the surface calls the IDXGIKeyedMutex::ReleaseSync method using the same value. This value can be any UINT64 value.</param>
    /// <param name="milliseconds">The time-out interval, in milliseconds. This method will return if the interval elapses, and the keyed mutex has not been released using the specified Key. If this value is set to zero, the AcquireSync method will test to see if the keyed mutex has been released and returns immediately. If this value is set to INFINITE, the time-out interval will never elapse.</param>
    /// <returns>Return S_OK if successful.
    /// If the owning device attempted to create another keyed mutex on the same shared resource, AcquireSync returns E_FAIL.
    /// AcquireSync can also return the following DWORD constants. Therefore, you should explicitly check for these constants. If you only use the SUCCEEDED macro on the return value to determine if AcquireSync succeeded, you will not catch these constants.
    /// WAIT_ABANDONED - The shared surface and keyed mutex are no longer in a consistent state. If AcquireSync returns this value, you should release and recreate both the keyed mutex and the shared surface.
    /// WAIT_TIMEOUT - The time-out interval elapsed before the specified key was released.
    /// </returns>
    public uint AcquireSync(ulong key, uint milliseconds)
    {
        return (*lpVtbl)->AcquireSync((IDXGIKeyedMutex*)lpVtbl, key, milliseconds);
    }

    /// <summary>
    /// Using a key, releases exclusive rendering access to a shared resource.
    /// </summary>
    /// <param name="key">A value that indicates which device to give access to. This method succeeds when the device that currently owns the surface calls the ReleaseSync method using the same value. This value can be any UINT64 value.</param>
    /// <returns>Returns S_OK if successful. If the device attempted to release a keyed mutex that is not valid or owned by the device, ReleaseSync returns E_FAIL.</returns>
    public uint ReleaseSync(ulong key)
    {
        const uint E_FAIL = unchecked(0x80004005);
        if (lpVtbl == null)
            return E_FAIL;

        return (*lpVtbl)->ReleaseSync((IDXGIKeyedMutex*)lpVtbl, key);
    }
}
