using System.Text.Json;
using Rrflow.Rrd;
using Xunit;

namespace Rrflow.Rrd.Client.Tests;

public sealed class SignalCatalogueTests
{
    private const string ExpectedOpenApiSha256 =
        "0ef644d8b65019d3fdbb3cf6bf5dd0961473d41d66b0080529801d0008cd9040";
    private const string ExpectedSignalSha256 =
        "239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07";

    [Fact]
    public void GeneratedSignalCatalogueIsPublicExactAndInert()
    {
        using JsonDocument document = JsonDocument.Parse(SignalCatalogue.Json);
        JsonElement catalogue = document.RootElement;

        Assert.Equal(ExpectedSignalSha256, SignalCatalogue.SignalCatalogueSha256);
        Assert.Equal(ExpectedOpenApiSha256, SignalCatalogue.OpenApiDocumentSha256);
        Assert.Equal(1, catalogue.GetProperty("contract_version").GetInt32());
        Assert.Equal(4, catalogue.GetProperty("diagnostic_levels").GetArrayLength());
        Assert.Equal(9, catalogue.GetProperty("metric_attributes").GetArrayLength());
        Assert.Equal(22, catalogue.GetProperty("metric_instruments").GetArrayLength());
        foreach (string forbidden in new[]
        {
            "activation", "active_level", "current_level", "diagnostic_snapshot", "enabled",
            "events", "exporter", "lifecycle", "samples", "subscriber", "timestamp",
        })
        {
            Assert.False(catalogue.TryGetProperty(forbidden, out _));
        }
    }
}
