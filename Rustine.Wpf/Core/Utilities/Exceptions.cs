using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.IO;

namespace Rustine.Wpf.Core.Utilities;

internal static class Exceptions
{
    public static void Assert(bool condition, string message = "")
    {
        Debug.Assert(condition, message);
    }

    public static void ThrowArgumentOutOfRangeIf(bool condition, string message = "")
    {
        if (condition)
        {
            throw new ArgumentOutOfRangeException(message);
        }
    }

    public static void ThrowInvalidOperationIf(bool condition, string message = "")
    {
        if (condition)
        {
            throw new InvalidOperationException(message);
        }
    }

    public static void ThrowIf(bool condition, string message = "")
    {
        if (condition)
        {
            throw new Exception(message);
        }
    }

    public static void ThrowIfNonZero(uint result, string message = "")
    {
        if (result != 0)
        {
            throw new Exception(message);
        }
    }

    public static T ThrowIfNull<T>([NotNull] T? instance, string message = "") where T : class
    {
        if (instance == null)
        {
            throw new Exception(message);
        }

        return instance;
    }

    public static T ThrowInvalidDataIfNull<T>([NotNull] T? instance, string message = "") where T : struct
    {
        if (instance == null)
        {
            throw new InvalidDataException(message);
        }
        return instance.Value;
    }

    public static T ThrowInvalidDataIfNull<T>([NotNull] T? instance, string message = "") where T : class
    {
        if (instance == null)
        {
            throw new InvalidDataException(message);
        }
        return instance;
    }

    public static string ThrowInvalidDataIfNullOrEmpty([NotNull] string? instance, string message = "")
    {
        if (string.IsNullOrWhiteSpace(instance))
        {
            throw new InvalidDataException(message);
        }
        return instance;
    }

    public static void ThrowInvalidDataIf(bool condition, string message = "")
    {
        if (condition)
        {
            throw new InvalidDataException(message);
        }
    }

    public static void ThrowNotSupportedIf(bool condition, string message = "")
    {
        if (condition)
        {
            throw new NotSupportedException(message);
        }
    }

    public static T ThrowNotSupportedIfNull<T>([NotNull] T? instance, string message = "") where T : class
    {
        if (instance == null)
        {
            throw new NotSupportedException(message);
        }
        return instance;
    }

    public static T ThrowNotSupportedIfDefault<T>(T instance, string message = "") where T : struct
    {
        if (instance.Equals(default(T)))
        {
            throw new NotSupportedException(message);
        }
        return instance;
    }
}