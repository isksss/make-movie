namespace Mm.Sdk;

/// <summary>
/// Minimal lifecycle contract generated from plugin-api/plugin.wit.
/// </summary>
public interface IMmPlugin
{
    PluginMetadata Metadata();

    void Initialize()
    {
    }

    void Shutdown()
    {
    }
}

public enum PluginCategory
{
    Ai,
    Subtitle,
    Tts,
    Template,
    Export,
    Utility,
}

public sealed record PluginMetadata(
    string Name,
    string Version,
    PluginCategory Category,
    string? DisplayName = null,
    string? Description = null);

public sealed class NoopPlugin : IMmPlugin
{
    public PluginMetadata Metadata() => new("noop-plugin", "0.1.0", PluginCategory.Utility);
}
