using System.Windows;
using System.Windows.Input;
using System.Windows.Threading;

using Rustine.Wpf.Core.Interop;

using static Rustine.Wpf.Core.Interop.DirectX;
using static Rustine.Wpf.Core.Interop.Windows;

namespace Rustine.Wpf;

/// <summary>
/// Interaction logic for MainWindow.xaml
/// </summary>
public partial class MainWindow : Window
{
    struct DirectXResources
    {
        public IDirect3D9Ex D3D9Instance;
        public IDirect3DDevice9Ex D3D9Device;

        public ID3D11Device D3D11Device;
        public ID3D11DeviceContext D3D11DeviceContext;

        public ulong AdapterLuid;

        public readonly bool IsValid()
        {
            return D3D9Device.IsValid && D3D11Device.IsValid;
        }

        public void Release()
        {
            D3D11DeviceContext.Release();
            D3D11Device.Release();

            D3D9Device.Release();
            D3D9Instance.Release();
        }
    }

    struct PresentationResources
    {
        public IDirect3DTexture9 WpfInteropTexture;
        public IDirect3DSurface9 WpfInteropSurface;
        public ID3D11Texture2D D3D9InteropTexture;
        public nint VulkanInteropHandle;

        public ID3D11Texture2D VulkanInteropTexture;
        public IDXGIKeyedMutex VulkanInteropMutex;

        public readonly bool IsValid()
        {
            return WpfInteropTexture.IsValid && WpfInteropSurface.IsValid && D3D9InteropTexture.IsValid && VulkanInteropHandle != 0;
        }

        public void Release()
        {
            D3D9InteropTexture.Release();
            VulkanInteropMutex.Release();
            VulkanInteropTexture.Release();
            WpfInteropSurface.Release();
            WpfInteropTexture.Release();
        }
    }

    readonly Core.Core core = new();

    DirectXResources directXResources;
    PresentationResources presentationResources;

    static readonly TimeSpan DeferRenderTime = TimeSpan.FromMilliseconds(50);
    readonly System.Diagnostics.Stopwatch renderingWatch = System.Diagnostics.Stopwatch.StartNew();
    TimeSpan lastRenderTime = TimeSpan.Zero;
    TimeSpan initializeRenderTime = TimeSpan.Zero;
    readonly DispatcherTimer deferRenderTimer = new() { Interval = DeferRenderTime + TimeSpan.FromMilliseconds(1) };

    Cursor? previousCursor;
    Point previousMouseOnWindow;
    Point previousMouseOnElement;
    bool isRawMouse = false;

    public MainWindow()
    {
        InitializeComponent();
    }

    async void Window_Loaded(object sender, RoutedEventArgs e)
    {
        InitializeDirectX();

        await core.Initialize(interopAdapterLuid: directXResources.AdapterLuid);
    }

    void Window_Closed(object sender, EventArgs e)
    {
        core.Dispose();

        presentationResources.Release();

#if DIRECTX_DEBUG
            {
                ID3D11Debug d3d11Debug;
                directXResources.D3D11Device.QueryInterface(out d3d11Debug);
                d3d11Debug.ReportLiveDeviceObjects(D3D11_RLDO_FLAGS.DETAIL | D3D11_RLDO_FLAGS.IGNORE_INTERNAL);
                d3d11Debug.Release();

                {
                    ID3D11InfoQueue d3d11InfoQueue;
                    directXResources.D3D11Device.QueryInterface(out d3d11InfoQueue);

                    var messageCount = d3d11InfoQueue.GetNumStoredMessages();
                    for (ulong i = 0; i < messageCount; i++)
                    {
                        string message = d3d11InfoQueue.GetMessage(i);
                        logger.Error($"{message}");
                    }
                    d3d11InfoQueue.ClearStoredMessages();
                    d3d11InfoQueue.Release();
                }
            }
#endif
        directXResources.Release();
    }

    void CompositionTarget_Rendering(object? sender, EventArgs e)
    {
#if WPF_DESIGN_TIME_BUILD
            System.Windows.Interop.D3DImage? presentTarget = null;
            if (presentTarget is null) return;
#endif

        if (!core.IsInitialized)
            return;

        if (!presentTarget.IsFrontBufferAvailable)
            return;

        if (!IsActive && (renderingWatch.Elapsed - lastRenderTime) < TimeSpan.FromMilliseconds(100))
            return;

        // NOTE: Disable dispatcher processing to avoid re-entrancy issues
        using (Application.Current.Dispatcher.DisableProcessing())
        {
            lastRenderTime = renderingWatch.Elapsed;
            core.Render();

            // If it has been a very short time since initialization, defer presenting to avoid high frequency flicker
            // A timer will re-trigger rendering after the defer period
            TimeSpan sinceInitialization = renderingWatch.Elapsed - initializeRenderTime;
            if (sinceInitialization >= DeferRenderTime)
            {
                deferRenderTimer.Stop();

                // TODO: Utilize mutex in WCR
                // Can avoid some swap locking in the main thread if render thread directly renders to the interop texture.
                // For now, we just acquire/release around the copy operation because it is needed by D3D11.
                presentationResources.VulkanInteropMutex.AcquireSync(key: 0, milliseconds: 0xFFFFFFFF);
                directXResources.D3D11DeviceContext.CopyResource(
                    dst: presentationResources.D3D9InteropTexture.Get(),
                    src: presentationResources.VulkanInteropTexture.Get());
                presentationResources.VulkanInteropMutex.ReleaseSync(key: 0);

                presentTarget.Lock();
                presentTarget.SetBackBuffer(System.Windows.Interop.D3DResourceType.IDirect3DSurface9, presentationResources.WpfInteropSurface.Get());
                presentTarget.AddDirtyRect(new Int32Rect(0, 0, presentTarget.PixelWidth, presentTarget.PixelHeight));
                presentTarget.Unlock();

#if DIRECTX_DEBUG
                    {
                        ID3D11InfoQueue d3d11InfoQueue;
                        directXResources.D3D11Device.QueryInterface(out d3d11InfoQueue);

                        var messageCount = d3d11InfoQueue.GetNumStoredMessages();
                        for (ulong i = 0; i < messageCount; i++)
                        {
                            string message = d3d11InfoQueue.GetMessage(i);
                            logger.Error($"{message}");
                        }
                        d3d11InfoQueue.ClearStoredMessages();
                        d3d11InfoQueue.Release();
                    }
#endif
            }
        }
    }

    void InitializeDirectX()
    {
#if WPF_DESIGN_TIME_BUILD
            Grid? presentElement = null;
            if (presentElement is null) return;
#endif

        ID3D11Device d3d11Device;
        ID3D11DeviceContext d3d11DeviceContext;
        CreateD3D11Device(
            pAdapter: 0,
            DriverType: D3D_DRIVER_TYPE.HARDWARE,
            Software: 0,
#if DIRECTX_DEBUG
                Flags: D3D11_CREATE_DEVICE_FLAG.BGRA_SUPPORT | D3D11_CREATE_DEVICE_FLAG.DEBUG,
#else
            Flags: D3D11_CREATE_DEVICE_FLAG.BGRA_SUPPORT,
#endif
            pFeatureLevels: 0,
            FeatureLevels: 0,
            SDKVersion: 7,
            ppDevice: out d3d11Device,
            ppImmediateContext: out d3d11DeviceContext);
        directXResources.D3D11Device = d3d11Device;
        directXResources.D3D11DeviceContext = d3d11DeviceContext;

        // Fetch adapter LUID
        IDXGIDevice dxgiDevice = default;
        IDXGIAdapter dxgiAdapter = default;

        try
        {
            d3d11Device.QueryInterface(out dxgiDevice);
            dxgiDevice.GetAdapter(out dxgiAdapter);

            DXGI_ADAPTER_DESC adapterDesc;
            dxgiAdapter.GetDesc(out adapterDesc);

            directXResources.AdapterLuid = adapterDesc.AdapterLuid.LowPart | ((ulong)adapterDesc.AdapterLuid.HighPart << 32);
        }
        finally
        {
            dxgiAdapter.Release();
            dxgiDevice.Release();
        }

        IDirect3D9Ex d3d9Instance;
        CreateD3D9Ex(32, out d3d9Instance);

        D3DPRESENT_PARAMETERS d3d9PresentParameters = new()
        {
            Windowed = 1,
            SwapEffect = 1,
            PresentationInterval = 2147483648u
        };

        IDirect3DDevice9Ex d3d9Device = default;
        d3d9Instance.CreateDeviceEx(
            adapter: 0,
            deviceType: D3DDEVTYPE.HAL,
            windowHandle: new System.Windows.Interop.WindowInteropHelper(this).Handle,
            behaviorFlags: D3DCREATE.MULTITHREADED | D3DCREATE.HARDWARE_VERTEXPROCESSING,
            presentParameters: ref d3d9PresentParameters,
            pDeviceEx: out d3d9Device);

        directXResources.D3D9Device = d3d9Device;
        directXResources.D3D9Instance = d3d9Instance;

        InitializeRender(presentWidth: (uint)presentElement.ActualWidth, presentHeight: (uint)presentElement.ActualHeight);
    }

    public void InitializeRender(uint presentWidth, uint presentHeight)
    {
        if (!directXResources.IsValid())
            return;

        // NOTE: Disable dispatcher processing to avoid re-entrancy issues
        using (Application.Current.Dispatcher.DisableProcessing())
        {
            PresentationResources localPresentation = presentationResources;

            presentationResources = CreatePresentSurface(presentWidth, presentHeight);

            core.InitializeRender(presentWidth, presentHeight, presentationResources.VulkanInteropHandle);

            localPresentation.Release();

            initializeRenderTime = renderingWatch.Elapsed;
            deferRenderTimer.Stop();
            deferRenderTimer.Start();
        }
    }

    PresentationResources CreatePresentSurface(uint width, uint height)
    {
        // 1. D3D9 texture for D3D9 -> WPF
        IDirect3DTexture9 wpfInteropTexture;
        nint wpfInteropTextureShared;
        IDirect3DSurface9 wpfInteropSurface;
        directXResources.D3D9Device.CreateTexture(
            width: width,
            height: height,
            levels: 1,
            usage: D3DUSAGE.RENDERTARGET,
            format: D3DFORMAT.A8R8G8B8,
            pool: 0,
            pTexture: out wpfInteropTexture,
            pSharedHandle: out wpfInteropTextureShared);
        wpfInteropTexture.GetSurfaceLevel(0, out wpfInteropSurface);

        // 2. D3D11 texture for D3D11 -> D3D9
        ID3D11Texture2D d3d9InteropTexture;
        directXResources.D3D11Device.OpenSharedResource(wpfInteropTextureShared, out d3d9InteropTexture);

        // 3. D3D11 texture for Vulkan -> D3D11
        D3D11_TEXTURE2D_DESC vulkanInteropTextureDesc = new()
        {
            Width = width,
            Height = height,
            MipLevels = 1,
            ArraySize = 1,
            Format = DXGI_FORMAT.B8G8R8A8_UNORM,
            SampleDesc = new DXGI_SAMPLE_DESC() { Count = 1, Quality = 0 },
            Usage = D3D11_USAGE.DEFAULT,
            BindFlags = D3D11_BIND_FLAG.RENDER_TARGET | D3D11_BIND_FLAG.SHADER_RESOURCE,
            CPUAccessFlags = D3D11_CPU_ACCESS_FLAG.NONE,
            MiscFlags = D3D11_RESOURCE_MISC_FLAG.SHARED_KEYEDMUTEX | D3D11_RESOURCE_MISC_FLAG.SHARED_NTHANDLE
        };
        ID3D11Texture2D vulkanInteropTexture;
        directXResources.D3D11Device.CreateTexture2D(vulkanInteropTextureDesc, default, out vulkanInteropTexture);

        // 4. Extract shared handle and keyed mutex for Vulkan interop
        IDXGIKeyedMutex vulkanInteropMutex;
        vulkanInteropTexture.QueryInterface(out vulkanInteropMutex);

        nint vulkanInteropHandle;
        {
            IDXGIResource1 vulkanInteropDxgiResource;
            vulkanInteropTexture.QueryInterface(out vulkanInteropDxgiResource);

            vulkanInteropDxgiResource.CreateSharedHandle(
                flags: DXGI_SHARED_RESOURCE_FLAGS.READ | DXGI_SHARED_RESOURCE_FLAGS.WRITE,
                sharedHandle: out vulkanInteropHandle);

            vulkanInteropDxgiResource.Release();
        }

        return new PresentationResources()
        {
            WpfInteropTexture = wpfInteropTexture,
            WpfInteropSurface = wpfInteropSurface,
            D3D9InteropTexture = d3d9InteropTexture,
            VulkanInteropHandle = vulkanInteropHandle,
            VulkanInteropTexture = vulkanInteropTexture,
            VulkanInteropMutex = vulkanInteropMutex,
        };
    }

    (double x, double y, double width, double height) Get3DViewport()
    {
#if WPF_DESIGN_TIME_BUILD
            Grid? presentViewport = null;
            if (presentViewport is null) return (0, 0, 0, 0);
#endif
        var position = presentViewport.TransformToAncestor(this)
                          .Transform(new Point(0, 0));
        return (position.X, position.Y, presentViewport.ActualWidth, presentViewport.ActualHeight);
    }

    void PresentViewport_Loaded(object sender, RoutedEventArgs e)
    {
        var (x, y, width, height) = Get3DViewport();
        core.SetRenderViewport((float)x, (float)y, (float)width, (float)height);
    }

    void PresentViewport_SizeChanged(object sender, SizeChangedEventArgs e)
    {
        var (x, y, width, height) = Get3DViewport();
        core.SetRenderViewport((float)x, (float)y, (float)width, (float)height);
    }

    void PresentElement_Loaded(object sender, RoutedEventArgs e)
    {
#if WPF_DESIGN_TIME_BUILD
            Grid? presentElement = null;
            if (presentElement is null) return;
#endif

        InitializeRender(presentWidth: (uint)presentElement.ActualWidth, presentHeight: (uint)presentElement.ActualHeight);
    }

    void PresentElement_SizeChanged(object sender, SizeChangedEventArgs e)
    {
        InitializeRender((uint)e.NewSize.Width, (uint)e.NewSize.Height);
    }

    void PresentElement_MouseDown(object sender, MouseButtonEventArgs e)
    {
#if WPF_DESIGN_TIME_BUILD
            Grid? presentElement = null;
            if (presentElement is null) return;
#endif

        FocusManager.SetFocusedElement(this, this);
        //wcr.SetFocusedView(UserFocusedView.Scene);

        if (e.ChangedButton == MouseButton.Middle)
        {
            presentElement.CaptureMouse();

            previousMouseOnWindow = e.MouseDevice.GetPosition(this);
            EnableRawMouseInput();

            previousCursor = e.MouseDevice.OverrideCursor;
            e.MouseDevice.OverrideCursor = Cursors.None;
        }

        HandleMouseButton(e);
    }

    void PresentElement_MouseUp(object sender, MouseButtonEventArgs e)
    {
#if WPF_DESIGN_TIME_BUILD
            Grid? presentElement = null;
            if (presentElement is null) return;
#endif

        if (isRawMouse && e.ChangedButton == MouseButton.Middle)
        {
            presentElement.ReleaseMouseCapture();

            var mouseOnScreen = PointToScreen(previousMouseOnWindow);
            SetCursorPos((int)mouseOnScreen.X, (int)mouseOnScreen.Y);

            e.MouseDevice.OverrideCursor = previousCursor;

            DisableRawMouseInput();
        }

        HandleMouseButton(e);
    }

    void PresentElement_MouseWheel(object sender, MouseWheelEventArgs e)
    {
        //wcr.MouseScroll(0, e.Delta);
    }

    void PresentElement_MouseMove(object sender, MouseEventArgs e)
    {
#if WPF_DESIGN_TIME_BUILD
            Grid? presentElement = null;
            if (presentElement is null) return;
#endif

        if (isRawMouse)
        {
            var mouseOnScreen = PointToScreen(previousMouseOnWindow);
            SetCursorPos((int)mouseOnScreen.X, (int)mouseOnScreen.Y);
            return;
        }

        Point mouseOnPresentElement = e.MouseDevice.GetPosition(presentElement);
        //wcr.MouseMove(new((float)mouseOnPresentElement.X, (float)mouseOnPresentElement.Y));

        previousMouseOnElement = mouseOnPresentElement;
    }

    void HandleMouseButton(MouseButtonEventArgs e)
    {
        /*InputButton button = e.ChangedButton switch
        {
            MouseButton.Left => InputButton.Left,
            MouseButton.Middle => InputButton.Middle,
            MouseButton.Right => InputButton.Right,
            MouseButton.XButton1 => InputButton.X4,
            MouseButton.XButton2 => InputButton.X5,
            _ => InputButton.Right,
        };

        InputAction action = e.ButtonState switch
        {
            MouseButtonState.Pressed => InputAction.Press,
            MouseButtonState.Released => InputAction.Release,
            _ => InputAction.Release,
        };*/

        //wcr.MouseButton(button, action);
    }

    unsafe void EnableRawMouseInput()
    {
        RAWINPUTDEVICE rawInputDevice = new()
        {
            usUsagePage = 0x01,
            usUsage = 0x02,
            dwFlags = 0,
            hwndTarget = new System.Windows.Interop.WindowInteropHelper(this).Handle,
        };
        _ = RegisterRawInputDevices(&rawInputDevice, 1, (uint)sizeof(RAWINPUTDEVICE));
        isRawMouse = true;
    }

    unsafe void DisableRawMouseInput()
    {
        RAWINPUTDEVICE rawInputDevice = new()
        {
            usUsagePage = 0x01,
            usUsage = 0x02,
            dwFlags = RAWINPUTDEVICE_FLAGS.RIDEV_REMOVE,
            hwndTarget = 0,
        };
        _ = RegisterRawInputDevices(&rawInputDevice, 1, (uint)sizeof(RAWINPUTDEVICE));
        isRawMouse = false;
    }

    protected override void OnSourceInitialized(EventArgs e)
    {
        base.OnSourceInitialized(e);

        System.Windows.Interop.HwndSource? source = PresentationSource.FromVisual(this) as System.Windows.Interop.HwndSource;
        source?.AddHook(WndProc);
    }

    unsafe IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg == 0x00FF)
        {
            uint size = 0;
            _ = GetRawInputData(lParam, RAW_INPUT_DATA_COMMAND_FLAGS.RID_INPUT, null, ref size, (uint)sizeof(RAWINPUTHEADER));

            RAWINPUT rawInput = new();
            _ = GetRawInputData(lParam, RAW_INPUT_DATA_COMMAND_FLAGS.RID_INPUT, &rawInput, ref size, (uint)sizeof(RAWINPUTHEADER));

            int dy, dx;
            if (rawInput.data.mouse.usFlags == 1)
            {
                dx = rawInput.data.mouse.lLastX - (int)previousMouseOnElement.X;
                dy = rawInput.data.mouse.lLastY - (int)previousMouseOnElement.Y;
            }
            else
            {
                dx = rawInput.data.mouse.lLastX;
                dy = rawInput.data.mouse.lLastY;
            }

            //wcr.MouseMove(new((float)previousMouseOnElement.X + dx, (float)previousMouseOnElement.Y + dy));

            previousMouseOnElement = new((float)previousMouseOnElement.X + dx, (float)previousMouseOnElement.Y + dy);
        }

        return IntPtr.Zero;
    }
}