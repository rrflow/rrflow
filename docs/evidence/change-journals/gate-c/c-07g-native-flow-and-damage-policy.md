# C-07g installed flow and explicit damage publication policy

**Status:** active implementation evidence; full native/soak qualification open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-native-flow-and-damage-policy`
**Owner:** [C-07f/g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting revision: 327e6f43. PR72 pre-edit comments 6362657720 and 6362688626
bind the bounded diagnostic follow-up and demonstrated damage-fixture correction.

## Observed flow and retained limits

Clean combined d470e59007a5ed0da0d0ffccb86d1f87abab9ace completed the actual
Windows Job characterization in 293.52s. The installed ordinary executable was
71afdaf9a66d288def318ca252b494953727f1c3abb2fac6bc342aa35757f868;
the harness was 698d6b4b180a6bef2a20afe5e0db5eb16ed4b0616682d357166f3290211e53de.
The frozen allocation was four CPUs/4 GiB on the 24-CPU, 68112736256-byte host.
It exercised 32 source records, 96 relations, 64 eight-dimensional vectors,
280 offered operations, all four insert/update/retire/recreate transitions,
two forced reopens, backup during writes, retained-reader ownership, drain,
three-minute soak, final restarted independent readback and owned-job cleanup.
All 28 raw segments completed with zero failures or unresolved operations.

Campaign SHA256: 6350724181c89805c89e629eb32249b341bcd2ba57652b8eb91bde76883a6a52.
Last segment: c89ed872cb5366290889444457b57696ae856771120335c4edebaadd1e2c16ab.
Raw local artifacts: `target/c07-windows-characterization-06/evidence`.
The new input explicitly allowed one configured 128 MiB active mutable buffer
of growth during drain; it still required zero projected growth, no debt, no
transaction/read-view/pin/queue owners and three stable samples. This did not
rewrite campaign05's failure or relax J-04 qualification. Rate was one request
per second, not the nominal profile. Qualification remains false.

The 96e30438 high-load campaign08 failed in 81.57s during the burst: sequence39
was a maintenance begin/abort operation, not a mixed write. Its closed cause was
StorageAdmission. The pre-edit comment's description as a burst write was
imprecise; the retained raw attempt is authoritative. The private workload
projection now distinguishes constant admission-family labels without emitting
server detail. No admission limit, outcome/retry rule or production policy is
changed. The separately frozen 24-hour small-corpus characterization07 started
on 96e30438 and remains pending; it cannot qualify the complete J-04 profile.

## Native CI failure and canonical correction

Run39932697981 at 868fa45dec7dbd20372b6d10f820cce025828900 completed all eight
real capacity cases on each platform: Windows job120345459867, macOS
job120345459675 and Linux job120345459691. These exact-candidate observations
are not transferred to later code. The same run's macOS job120345459745 and
Linux job120345459868 failed both damage modes at damage.rs:123: the fixture
assumed application backup would publish a physical segment. The corrected
backup intentionally reads one native transaction without that flush side effect.

`support/sustained/damage.rs::run` now creates a typed TOML fixture policy from
the bundled defaults, changes only its soft WAL flush threshold to one byte,
and uses `Estate::install_configured` with explicit plan/apply digest binding.
It asserts the effective installed threshold. Ordinary writes publish genuinely
reachable segments. `target` still selects them from the physical inspector;
no file is fabricated and no missing/corrupt-byte case is skipped. Production
defaults, hard caps, read-only backup behavior, inventory equality, denial,
restoration and exact receipt replay remain unchanged.

Reviewed: full damage driver, Estate installation/start, workload diagnostics
and maintenance branch, service/store error mappings and display, transaction
rejection/abort handling, default configuration, existing crash fixture policy,
raw failed segments and the named CI logs. Existing behavior is retained at its
canonical fixture boundary; nothing was moved/deleted. No research or upstream
source adaptation is needed. Stop on weakened denial/readback, implicit repair,
production default changes, or raw diagnostic disclosure.

## Validation and remaining work

The expanded closed-label regression passed (one test, 0.00s); formatting passed.
The fresh installed damage matrix passed (one test, 363.29s): all eight
missing/corrupt roles denied access without mutation, exact saved bytes restored
all five acknowledgements/replays, and application restore preserved the four
baseline records/eight relations/eight vectors, denied old session authority,
excluded the later batch and admitted/replayed new work after restart. Command:
`cargo test --locked -p rrflow-cli --test installed_reopen -- sustained::damage::fresh_installed_damage_preserves_evidence_and_acknowledged_state --exact --ignored --nocapture`.
Existing-mode package validation and updated candidate CI remain required. The combined evaluator suite passed in
an isolated target before these fixture-only corrections. Changed paths:
workload diagnostics, damage fixture, this record, POAM-035, generated indexes
and inventory. No roadmap completion or alpha status changes.
