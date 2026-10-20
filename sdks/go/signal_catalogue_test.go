package rrd

import (
	"encoding/json"
	"testing"
)

const expectedOpenAPISHA256 = "52cee221bf6d1c22b8146fd490cfd080af764b21270ca50caee6a5abce21de76"
const expectedSignalCatalogueSHA256 = "239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07"

func TestGeneratedSignalCatalogueIsPublicExactAndInert(t *testing.T) {
	var catalogue struct {
		ContractVersion   int              `json:"contract_version"`
		DiagnosticLevels  []string         `json:"diagnostic_levels"`
		MetricAttributes  []string         `json:"metric_attributes"`
		MetricInstruments []map[string]any `json:"metric_instruments"`
	}
	if err := json.Unmarshal([]byte(SignalCatalogueJSON), &catalogue); err != nil {
		t.Fatalf("decode generated signal catalogue: %v", err)
	}

	if SignalCatalogueSHA256 != expectedSignalCatalogueSHA256 {
		t.Fatalf("signal catalogue digest = %s", SignalCatalogueSHA256)
	}
	if OpenAPIDocumentSHA256 != expectedOpenAPISHA256 {
		t.Fatalf("OpenAPI digest = %s", OpenAPIDocumentSHA256)
	}
	if catalogue.ContractVersion != 1 || len(catalogue.DiagnosticLevels) != 4 ||
		len(catalogue.MetricAttributes) != 9 || len(catalogue.MetricInstruments) != 22 {
		t.Fatalf("unexpected signal catalogue shape: %+v", catalogue)
	}

	var topLevel map[string]json.RawMessage
	if err := json.Unmarshal([]byte(SignalCatalogueJSON), &topLevel); err != nil {
		t.Fatalf("decode generated signal catalogue keys: %v", err)
	}
	for _, forbidden := range []string{
		"activation", "active_level", "current_level", "diagnostic_snapshot", "enabled",
		"events", "exporter", "lifecycle", "samples", "subscriber", "timestamp",
	} {
		if _, exists := topLevel[forbidden]; exists {
			t.Fatalf("signal discovery contains runtime key %q", forbidden)
		}
	}
}
