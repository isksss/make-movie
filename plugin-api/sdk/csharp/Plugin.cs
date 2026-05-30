namespace Mm.Sdk;

/// <summary>
/// Minimal lifecycle contract defined by plugin-api/plugin.wit.
/// </summary>
public interface IMmPlugin
{
    string Metadata();

    void Initialize()
    {
    }

    void Shutdown()
    {
    }
}

public sealed class NoopPlugin : IMmPlugin
{
    public string Metadata() => "{}";
}
