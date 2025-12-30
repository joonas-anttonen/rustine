using System.Diagnostics;
using System.Text;

using Rustine.Wpf.Core.Api;

namespace Rustine.Wpf.Core;

class Core : IDisposable
{
    bool isDisposed;
    bool isInitialized;

    readonly Log log = new();
    RustineLogCallback? logCallback;

    public bool IsInitialized => isInitialized;
    public Log Log => log;

    public async Task Initialize(ulong interopAdapterLuid)
    {
        Debug.Assert(!isInitialized, "Core is already initialized.");
        isInitialized = true;

        log.Debug<Core>(nameof(Initialize));
        logCallback = OnRustineEvent;

        StartupRustine(interopAdapterLuid);
    }

    void StartupRustine(ulong interopAdapterLuid)
    {
        unsafe
        {
            string hostName = "Rustine.Wpf";
            Span<byte> hostNameBytes = stackalloc byte[1024];
            int hostNameLength = Encoding.UTF8.GetBytes(hostName.AsSpan(), hostNameBytes);
            hostNameBytes[Math.Min(hostNameLength, hostNameBytes.Length - 1)] = 0;

            var startupParameters = new StartupParameters
            {
                Callback = System.Runtime.InteropServices.Marshal.GetFunctionPointerForDelegate(logCallback!),
                EnableDebugging = 1,
                PhysicalDeviceId = interopAdapterLuid,
                HostPlatform = Platform.Windows,
                HostVersion = new Api.Version(0, 1, 0),
                HostName = (nint)System.Runtime.CompilerServices.Unsafe.AsPointer(ref hostNameBytes[0])
            };

            Status status = Api.Api.Startup((nint)(&startupParameters));
            if (status != Status.Success)
            {
                log.Error<Core>($"Failed to start Rustine core. Status: {status}");
            }
        }
    }

    void ShutdownRustine()
    {
        Status status = Api.Api.Shutdown();
        if (status != Status.Success)
        {
            log.Error<Core>($"Failed to shut down Rustine core. Status: {status}");
        }
    }

    void OnRustineEvent(in FfiEvent ffiEvent)
    {
        try
        {
            var severity = ffiEvent.Severity switch
            {
                0 => Log.Severity.Debug,
                1 => Log.Severity.Information,
                2 => Log.Severity.Warning,
                3 => Log.Severity.Error,
                _ => Log.Severity.Information,
            };

            string message = string.Empty;
            string thread = "unknown";
            string origin = "unknown";
            unsafe
            {
                if (ffiEvent.Message != nint.Zero && ffiEvent.MessageLength > 0)
                {
                    message = Encoding.UTF8.GetString((byte*)ffiEvent.Message, (int)ffiEvent.MessageLength);
                }
                if (ffiEvent.Thread != nint.Zero && ffiEvent.ThreadLength > 0)
                {
                    thread = Encoding.UTF8.GetString((byte*)ffiEvent.Thread, (int)ffiEvent.ThreadLength);
                }
                if (ffiEvent.Origin != nint.Zero && ffiEvent.OriginLength > 0)
                {
                    origin = Encoding.UTF8.GetString((byte*)ffiEvent.Origin, (int)ffiEvent.OriginLength);
                }
            }

            var epochTime = new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);
            var timestamp = epochTime.AddSeconds(ffiEvent.TimestampSecs).AddMilliseconds(ffiEvent.TimestampNanos / 1_000_000.0);

            var logEvent = new Log.Event(severity, message, timestamp, thread, origin);
            log.Append(logEvent);
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Error in FFI log callback: {ex.Message}");
        }
    }

    protected virtual void Dispose(bool disposing)
    {
        if (isDisposed && disposing)
        {
            //logger.Warning("Core.Dispose called multiple times");
        }

        if (!isDisposed)
        {
            isDisposed = true;

            if (isInitialized)
            {
                ShutdownRustine();
                isInitialized = false;
            }
        }
    }

    ~Core()
    {
        Dispose(disposing: false);
    }

    public void Dispose()
    {
        Dispose(disposing: true);
        GC.SuppressFinalize(this);
    }

    public void InitializeRender(uint presentWidth, uint presentHeight, nint presentTextureHandle)
    {
        if (!IsInitialized)
            return;

        unsafe
        {
            var renderParameters = new PresentationParameters
            {
                Width = presentWidth,
                Height = presentHeight,
                SurfaceHandle = presentTextureHandle,
                SurfaceSyncHandle = nint.Zero
            };

            Status status = Api.Api.InitializePresentation((nint)(&renderParameters));
            if (status != Status.Success)
            {
                log.Error<Core>($"Failed to initialize Rustine presentation. Status: {status}");
            }
        }

        //scene.InitializePresenting(
        //    format: Interop.Vulkan.Format.B8G8R8A8Unorm,
        //    width: presentWidth,
        //    height: presentHeight,
        //    handle: presentTextureHandle,
        //    clearColor: new Color(ColorTheme.Background.R, ColorTheme.Background.G, ColorTheme.Background.B));
        //scene.InitializeRendering(renderParameters);
    }

    public void SetRenderViewport(float x, float y, float width, float height)
    {
        if (!IsInitialized)
            return;

        //scene.SetViewport(x, y, width, height);
    }

    public void Render()
    {

    }
}