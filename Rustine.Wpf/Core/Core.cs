using System.Diagnostics;

namespace Rustine.Wpf.Core;

class Core : IDisposable
{
    bool isDisposed;
    bool isInitialized;

    readonly Log log = new();

    public bool IsInitialized => isInitialized;

    public async Task Initialize(ulong interopAdapterLuid)
    {
        Debug.Assert(!isInitialized, "Core is already initialized.");
        isInitialized = true;

        Api.Api.Initialize();
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