using System.Diagnostics;
using System.Text;

namespace Rustine.Wpf.Core;

class Core : IDisposable
{
    bool isDisposed;
    bool isInitialized;

    readonly Log log = new();
    Api.FfiLogCallback? logCallback;

    public bool IsInitialized => isInitialized;

    public async Task Initialize(ulong interopAdapterLuid)
    {
        Debug.Assert(!isInitialized, "Core is already initialized.");
        isInitialized = true;

        logCallback = OnRustLogEvent;
        Api.Api.Initialize(logCallback);
    }

    private void OnRustLogEvent(in Api.FfiEvent ffiEvent)
    {
        try
        {
            // Convert severity
            var severity = ffiEvent.Severity switch
            {
                0 => Log.Severity.Debug,
                1 => Log.Severity.Information,
                2 => Log.Severity.Warning,
                3 => Log.Severity.Error,
                _ => Log.Severity.Information,
            };

            // Convert message from unmanaged memory
            string message = string.Empty;
            unsafe
            {
                if (ffiEvent.Message != nint.Zero && ffiEvent.MessageLength > 0)
                {
                    message = Encoding.UTF8.GetString((byte*)ffiEvent.Message, (int)ffiEvent.MessageLength);
                }
            }

            // Convert thread name from unmanaged memory
            string threadName = "unknown";
            unsafe
            {
                if (ffiEvent.Thread != nint.Zero && ffiEvent.ThreadLength > 0)
                {
                    threadName = Encoding.UTF8.GetString((byte*)ffiEvent.Thread, (int)ffiEvent.ThreadLength);
                }
            }

            // Reconstruct DateTime from seconds and nanos
            var epochTime = new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);
            var timestamp = epochTime.AddSeconds(ffiEvent.TimestampSecs).AddMilliseconds(ffiEvent.TimestampNanos / 1_000_000.0);

            // Create a log event and append it
            var logEvent = new Log.Event(severity, message, timestamp, threadName);
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
                Api.Api.Terminate();
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