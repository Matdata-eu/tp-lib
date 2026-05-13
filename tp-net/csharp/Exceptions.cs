namespace TpLib;

/// <summary>Base exception type for all tp-lib failures.</summary>
public class TpLibException : Exception
{
    public TpLibException(string message) : base(message) { }
    public TpLibException(string message, Exception inner) : base(message, inner) { }
}

public sealed class TpLibIoException : TpLibException
{
    public TpLibIoException(string message) : base(message) { }
    public TpLibIoException(string message, Exception inner) : base(message, inner) { }
}

public sealed class TpLibParseException : TpLibException
{
    public TpLibParseException(string message) : base(message) { }
    public TpLibParseException(string message, Exception inner) : base(message, inner) { }
}

public sealed class TpLibConfigurationException : TpLibException
{
    public TpLibConfigurationException(string message) : base(message) { }
    public TpLibConfigurationException(string message, Exception inner) : base(message, inner) { }
}

public class TpLibProjectionException : TpLibException
{
    public TpLibProjectionException(string message) : base(message) { }
    public TpLibProjectionException(string message, Exception inner) : base(message, inner) { }
}

public sealed class NoMatchWithinRadiusException : TpLibProjectionException
{
    public NoMatchWithinRadiusException(string message) : base(message) { }
    public NoMatchWithinRadiusException(string message, Exception inner) : base(message, inner) { }
}

public class TpLibPathException : TpLibException
{
    public TpLibPathException(string message) : base(message) { }
    public TpLibPathException(string message, Exception inner) : base(message, inner) { }
}

public sealed class NoNavigablePathException : TpLibPathException
{
    public NoNavigablePathException(string message) : base(message) { }
    public NoNavigablePathException(string message, Exception inner) : base(message, inner) { }
}

public sealed class TpLibDetectionException : TpLibException
{
    public TpLibDetectionException(string message) : base(message) { }
    public TpLibDetectionException(string message, Exception inner) : base(message, inner) { }
}
