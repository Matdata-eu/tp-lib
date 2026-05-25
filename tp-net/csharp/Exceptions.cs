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

/// <summary>Base class for ERA RINF topology retrieval failures (feature 006).</summary>
public class TpLibRinfException : TpLibException
{
    public TpLibRinfException(string message) : base(message) { }
    public TpLibRinfException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>GNSS input was invalid (empty, mixed CRS, etc.) for auto retrieval.</summary>
public sealed class TpLibInvalidGnssInputException : TpLibRinfException
{
    public TpLibInvalidGnssInputException(string message) : base(message) { }
    public TpLibInvalidGnssInputException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>The SPARQL endpoint returned an HTTP / transport / parse failure.</summary>
public sealed class TpLibRinfRetrievalFailedException : TpLibRinfException
{
    public TpLibRinfRetrievalFailedException(string message) : base(message) { }
    public TpLibRinfRetrievalFailedException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>Some GNSS positions fall outside the retrieved RINF coverage.</summary>
public sealed class TpLibRinfMissingCoverageException : TpLibRinfException
{
    public TpLibRinfMissingCoverageException(string message) : base(message) { }
    public TpLibRinfMissingCoverageException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>Retrieved topology is structurally incomplete (e.g. coarse geometry, zero netrelations).</summary>
public sealed class TpLibRinfIncompleteTopologyException : TpLibRinfException
{
    public TpLibRinfIncompleteTopologyException(string message) : base(message) { }
    public TpLibRinfIncompleteTopologyException(string message, Exception inner) : base(message, inner) { }
}
