import assert from "node:assert/strict";
import test from "node:test";
import { ENDPOINT_OPENAPI_DOCUMENT_SHA256 } from "../src/generated/endpoints.js";
import {
  OPENAPI_DOCUMENT_SHA256,
  SIGNAL_CATALOGUE,
  SIGNAL_CATALOGUE_JSON,
  SIGNAL_CATALOGUE_SHA256,
} from "../src/index.js";

const EXPECTED_SIGNAL_SHA256 = "239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07";

test("generated signal catalogue is public, exact, and inert", () => {
  const parsed = JSON.parse(SIGNAL_CATALOGUE_JSON) as {
    contract_version: number;
    diagnostic_levels: unknown[];
    metric_attributes: unknown[];
    metric_instruments: unknown[];
  };

  assert.equal(SIGNAL_CATALOGUE_SHA256, EXPECTED_SIGNAL_SHA256);
  assert.equal(OPENAPI_DOCUMENT_SHA256, ENDPOINT_OPENAPI_DOCUMENT_SHA256);
  assert.deepEqual(SIGNAL_CATALOGUE, parsed);
  assert.equal(parsed.contract_version, 1);
  assert.equal(parsed.diagnostic_levels.length, 4);
  assert.equal(parsed.metric_attributes.length, 9);
  assert.equal(parsed.metric_instruments.length, 22);
  assert.equal(Object.isFrozen(SIGNAL_CATALOGUE), true);
  assert.equal(Object.isFrozen(SIGNAL_CATALOGUE.metric_instruments), true);
  assert.equal(Object.isFrozen(SIGNAL_CATALOGUE.metric_instruments[0]), true);
  assert.doesNotMatch(
    SIGNAL_CATALOGUE_JSON,
    /"(?:activation|active_level|current_level|diagnostic_snapshot|enabled|events|exporter|lifecycle|samples|subscriber|timestamp)"\s*:/,
  );
});
