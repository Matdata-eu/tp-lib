namespace TpLib;

public enum DetectionKind
{
    Punctual,
    Linear,
}

public enum Navigability
{
    Both,
    Forward,
    Backward,
    None,
}

public enum PathCalculationMode
{
    TopologyBased,
    FallbackIndependent,
}

public enum PathOrigin
{
    Algorithm,
    Manual,
}
