# Experimental controller testing bundle

**Status:** active Windows engineering artifact; model reliability and release qualification open
**Coordinate:** `rrflow://rrflow-instance/data/reference/deployment/controller-testing-bundle`
**Owner:** installed-lifecycle engineering delivery; J-03/J-05 remain open

This optional Windows x64 archive packages the primary engine and explicitly
separate controller trial tools. It is unsigned and unqualified. It does not
contain model weights, credentials, an engram, a task or a preselected provider.
The [installed lifecycle](installed-lifecycle.md) remains the lifecycle owner;
the [release gate](../../roadmap/rrflow-1.0/gate-j.md) owns default distribution
qualification. Neither archive assembly nor a model call closes that gate.

## Assemble from identified inputs

From a clean tracked checkout, build the primary executable with
`python scripts/build_rrflow.py --profile release --target-dir <build-directory>
--output <new-receipt.json>`. Then run:

```powershell
python scripts/package_controller_trial.py --build-receipt <receipt.json> --target-dir <build-directory> --output <new-archive.zip>
```

The assembler checks the executable's actual version report and SHA256 against
the receipt, requires unchanged runtime/contract source, and compiles the optional
`endpoint_selection` contract helper. It records engine and assembly revisions
separately. The manifest enumerates every member; publication never overwrites an
existing archive/checksum. A receipt's portable binary locator changes, while its
embedded build report is preserved exactly. Assembly requires Python 3.11+, Git
and the Rust build tools; using the primary executable requires none of them.

## Verify and test

Compare the archive digest delivered separately, then run the verifier:

```powershell
python scripts/verify_controller_trial.py <archive.zip> --sha256 <expected-sha256>
```

It checks exact members, bounds, paths, hashes and binary receipt consistency
without extracting or executing. Hashes establish byte identity, not publisher
authentication. The standalone [packaged runbook](../../../scripts/inference/controller_trial_README.txt)
gives primary install/serve/ready/verify commands and optional real endpoint
trials. Those trials require Python 3.11+, explicit profile and real task inputs,
and private credentials outside the archive. Recovery reads the existing attempt
without another model call. Preserve uncertain attempts and original receipts.

The [implementation journal](../../evidence/change-journals/repository/controller-testing-bundle-tooling.md)
records the tooling. The [actual artifact journal](../../evidence/change-journals/repository/controller-testing-bundle-artifact.md)
records verified relocation, a real BF16 proposal/reopen and primary readiness,
alongside an actual invalid model effort choice rejected by the engine. This is
a usable engineering trial, not a qualified model or default release.
The [refreshed artifact record](../../evidence/change-journals/repository/controller-bundle-refreshed-artifact.md)
binds the later combined-engine archive to its own release build, 18 integrity
checks and relocated native install/restart verification. It includes no new
model observation; earlier BF16 results do not transfer to the refreshed binary.
Engine-owned worker execution, source-edit
replanning, connected UI, routing quality, training permission, other platforms,
signed releases and container delivery require their own evidence.
