using System.Reflection;
using System.Runtime.InteropServices;

namespace TpLib;

internal static class TpLibNative
{
    internal const string LibName = "tp_lib_net";

    static TpLibNative()
    {
        NativeLibrary.SetDllImportResolver(typeof(TpLibNative).Assembly, Resolve);
    }

    /// <summary>Ensures the static constructor (and resolver registration) has run.</summary>
    internal static void EnsureInitialized()
    {
        // no-op; touching the type triggers the static ctor.
    }

    internal static unsafe void FreeByteBuffer(ByteBuffer buf)
    {
        NativeMethods.tp_net_free_byte_buffer(buf);
    }

    private static IntPtr Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (libraryName != LibName)
        {
            return IntPtr.Zero;
        }

        var rid = GetRuntimeIdentifier();
        var fileName = GetNativeFileName(libraryName);
        var asmDir = Path.GetDirectoryName(assembly.Location) ?? AppContext.BaseDirectory;

        // Candidate paths (in order):
        //  1) runtimes/{rid}/native/{file}  (NuGet layout)
        //  2) {asmDir}/{file}               (loose dev/test layout)
        //  3) {AppContext.BaseDirectory}/{file}
        var candidates = new[]
        {
            Path.Combine(asmDir, "runtimes", rid, "native", fileName),
            Path.Combine(asmDir, fileName),
            Path.Combine(AppContext.BaseDirectory, fileName),
        };

        foreach (var path in candidates)
        {
            if (File.Exists(path) && NativeLibrary.TryLoad(path, out var handle))
            {
                return handle;
            }
        }

        // Fall back to default OS search.
        return NativeLibrary.TryLoad(fileName, assembly, searchPath, out var fallback)
            ? fallback
            : IntPtr.Zero;
    }

    private static string GetRuntimeIdentifier()
    {
        var arch = RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            Architecture.X86 => "x86",
            _ => "x64",
        };
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows)) return $"win-{arch}";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux)) return $"linux-{arch}";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX)) return $"osx-{arch}";
        return $"unknown-{arch}";
    }

    private static string GetNativeFileName(string baseName)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows)) return $"{baseName}.dll";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX)) return $"lib{baseName}.dylib";
        return $"lib{baseName}.so";
    }
}
