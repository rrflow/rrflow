RRFlow experimental controller testing bundle

This is an unsigned, unqualified Windows x64 engineering build. It contains the
engine, not model weights. Model access is optional and explicitly configured.
The controller proposes a binding; worker execution, project-edit planning,
Connectome integration and automatic learning are not supplied by this bundle.

CHECK THE DOWNLOAD

Compare the ZIP's SHA256 with the digest delivered separately:
  Get-FileHash .\<archive>.zip -Algorithm SHA256

With Python 3.11+, the included verify_controller_trial.py can verify every
member, using that separately supplied archive digest. It does not extract or
execute archive contents:
  python .\verify_controller_trial.py .\<archive>.zip --sha256 <expected-digest>

Hashes identify bytes; this test bundle has no platform signature or release
attestation. LICENSE contains the distribution terms. trial-manifest.json records
the exact engine and assembly source identities and every packaged file hash.

START THE ENGINE (PowerShell, from the extracted bundle)

Choose a project you own. Use an empty project for the first test. The primary
binary needs no Python, Rust compiler, source checkout or model for this flow.

  $project = Read-Host 'Absolute project directory'
  $identity = (.\rrflow.exe --json install identity | ConvertFrom-Json).installation_identity
  $preview = .\rrflow.exe --json install plan --project $project --mode existing --identity $identity
  [IO.File]::WriteAllText((Join-Path $PWD 'installation-plan.json'), ($preview -join "`n"), [Text.UTF8Encoding]::new($false))

Read installation-plan.json before applying. It is the exact change preview:
  $plan = Get-Content -Raw .\installation-plan.json | ConvertFrom-Json
  .\rrflow.exe --json install apply --project $project --mode existing --plan .\installation-plan.json --expect $plan.installation.plan_sha256
  .\rrflow.exe serve --project $project --bind 127.0.0.1:0

In a second PowerShell window, from this bundle:
  .\rrflow.exe --json ready --project <same-project-directory>

The operating system chooses a free loopback port. ready authenticates and
returns the current engine discovery endpoints. Use its
public catalogue/schemas for client operations. Stop the owned serve process with
Ctrl+C. Then, while the engram is stopped:
  .\rrflow.exe --json verify --project <same-project-directory> --level quick

Restart with the same serve command. Do not delete the engram to recover a failed
attempt. Retain the exact executable for that engram: installation binds its
binary identity. Changing a binary is not an implicit engram migration.

OPTIONAL: ONE REAL CONTROLLER TRIAL

This separately packaged proof tool requires Python 3.11+ and an available model
endpoint. It creates a NEW isolated installed engram in an absent output folder,
then runs one real proposal, restarts, verifies exact readback and rejects replay.
It does not run a worker, change your project, train a model or label your outcome.

Supply all inputs yourself; no task, endpoint, model or credential is preselected:
  --profile: your EndpointProfileInput JSON with explicit experimental selection
             policy, eligible bindings, prompt, limits and credential references.
  --task-source: JSON with title, body and url from an actual task/source. The
                 bounded body projection and its original hash are recorded.
  --secret-file: private KEY=VALUE file, outside this bundle and output archive.
  --secret-name: the key name in that private file.
  --environment-name: the process-only name used by the explicit credential map.

  python .\tools\installed_controller_proof.py --build-receipt .\build-receipt.json --contract-example .\tools\endpoint_selection.exe --profile <profile.json> --task-source <task.json> --secret-file <private.env> --secret-name <key-name> --environment-name <process-name> --output <new-output-directory>

The helper is an optional offline contract tool, not a second engine. It derives
request digests using the same Rust contract. No build tool or checkout is needed.
No model endpoint, key, dataset, sample output or private engram is in this ZIP.

DIAGNOSE AND RECOVER

result.json appears only when the requested proof completes. Inspect retained
*-intent.json, response JSON and owned-process records after a failure. An absent
result does not authorize repeating a model request. To read the existing attempt
and budget without provider credentials or dispatch:

  python .\tools\installed_controller_proof.py --recover-proof <previous-output-directory> --output <new-readback-directory>

Uncertain remote execution stays uncertain; this package has no public operation
to clear it. Preserve the original directory and report its actual response.
Never upload project credential files, private environment files or engram data.

--admission-only exercises installed denials with no provider credentials. It
requires the four build/helper/profile/task input flags and an absent output path.
--fault-case configuration|session|stop|disconnect performs explicit additional
real calls with controlled response delivery. Configuration uses two calls; other
fault cases use one. Run these only within your authorized provider budget.
Held-response Stop proves rejection of local acceptance, not remote GPU shutdown.

Record actual source/manifest, task, response, failures and verification results
when reporting a test. Do not turn a successful transport test into a routing-
quality label or training permission. Further training requires reviewed real data.
