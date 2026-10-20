package io.rrflow.rrd;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import org.junit.jupiter.api.Test;
import tools.jackson.databind.JsonNode;
import tools.jackson.databind.json.JsonMapper;

final class SignalCatalogueTest {
    private static final JsonMapper JSON = JsonMapper.builder().build();
    private static final String EXPECTED_SIGNAL_SHA256 =
            "239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07";

    @Test
    void generatedSignalCatalogueIsPublicExactAndInert() throws Exception {
        JsonNode catalogue = JSON.readTree(SignalCatalogue.JSON);

        assertEquals(EXPECTED_SIGNAL_SHA256, SignalCatalogue.SIGNAL_CATALOGUE_SHA256);
        assertEquals(OperationId.OPENAPI_DOCUMENT_SHA256, SignalCatalogue.OPENAPI_DOCUMENT_SHA256);
        assertEquals(1, catalogue.path("contract_version").asInt());
        assertEquals(4, catalogue.path("diagnostic_levels").size());
        assertEquals(9, catalogue.path("metric_attributes").size());
        assertEquals(22, catalogue.path("metric_instruments").size());
        for (String forbidden : new String[] {
            "activation", "active_level", "current_level", "diagnostic_snapshot", "enabled",
            "events", "exporter", "lifecycle", "samples", "subscriber", "timestamp"
        }) {
            assertFalse(catalogue.has(forbidden), "signal discovery contains " + forbidden);
        }
    }
}
