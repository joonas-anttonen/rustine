using System.Runtime.InteropServices;

namespace Rustine.Wpf.Core.Api;

static partial class Api
{
    const string LibraryName = "rustine";

    [LibraryImport(LibraryName, EntryPoint = "initialize")]
    public static partial void Initialize();
}