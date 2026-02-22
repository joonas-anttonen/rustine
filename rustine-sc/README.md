# rustine-sc

DXC interop crate used to compile HLSL to SPIR-V.

## Windows setup

`rustine-sc` requires access to DXC headers and the `dxcompiler.dll` runtime.

Set one of these:

- `DXC_INCLUDE_DIR` to a directory containing either `dxc/dxcapi.h` or `dxcapi.h`
- `DXC_LIB_DIR` to a directory containing `dxcompiler.dll` (build script also tries `../include` relative to this path for headers)

Typical PowerShell example:

```powershell
$env:DXC_LIB_DIR = "C:\\SDKs\\DXC\\bin\\x64"
$env:DXC_INCLUDE_DIR = "C:\\SDKs\\DXC\\include"
cargo build -p rustine-sc
```

If only `DXC_LIB_DIR` is set, `build.rs` will attempt to locate headers in `../include` automatically.
