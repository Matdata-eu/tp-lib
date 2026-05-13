using System;
using System.IO;

namespace TpLib.Tests;

internal static class TestData
{
    private static readonly Lazy<string> _root = new(FindRoot);

    public static string Root => _root.Value;

    public static string Path(string relative) => System.IO.Path.Combine(Root, relative);

    public static string Read(string relative) => File.ReadAllText(Path(relative));

    private static string FindRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null)
        {
            var candidate = System.IO.Path.Combine(dir.FullName, "test-data");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }
            dir = dir.Parent;
        }
        throw new DirectoryNotFoundException("Could not locate test-data directory");
    }
}
