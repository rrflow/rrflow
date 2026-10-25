# C-07: retained native descriptor exhaustion

**Status:** active failure evidence; sustained qualification remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-native-descriptor-failure`
**Owner:** OBJ-02 / C-07g / POAM-035 / PR128 and integrated PR129

The [encoder package](c-07-encoder-working-set.md) reduced its measured working
memory while preserving encoded bytes. Its full Desktop preflight still failed.
Frozen source `9957ed3960d0c31a8cc91a4b1cd78b4d883679b3`, tree
`03673051992f7c7948e701a49535e6d1d12aed91`, binary
`f839e4115373ebb268d4f1bcf2ef5378fbe40fbe11679a4fd9c67f34f058ee60` and harness
`f2099f58208b30694b3b9d5fd8c3bbac78c150707aeafacea2821d1ab3b441fd` remain the
identity of this failed run, not the later integrated candidate.

The workload began at 2026-10-25 01:24:33 UTC. It reached 77,945 acknowledged
records out of 77,952 issued intents, then failed after 2,868,524 ms. The launcher
recorded failed status, exit 101 and confirmed cleanup. The continuation refused
qualification. No actual 24-hour soak started; no threshold or deadline changed.

The recorded failure was `api-internal-storage-substrate` with digest
`3cb203a45f9475dee0d9d3532ae8893c8629b525d4dd7bcfbc7edf5fb2125860`.
Reading the canonical client formatter and evaluator digest grammar, independent
Python SHA-256 of domain `c07g.client-error`, a zero separator and compact JSON
of the following synthetic error exactly reproduces that digest:

```text
RRD API 500 Internal Server Error: Internal: Storage("substrate: RRD LSM I/O: Too many open files (os error 24)")
```

The actual `systemctl --user show` record for the failed launcher reports
`LimitNOFILESoft=1024`, hard limit 524288 and exit status 1. An earlier query
without `--user` selected the wrong service manager; its default values were
discarded. The retained estate inventories 977 storage files. The immutable
`PageSource::File` implementation holds one strong `Arc<File>` per segment,
explaining descriptor growth independently of the encoder's allocation repair.

Read-only `rrflow verify --project <retained-project> --json` exited zero and
passed all 13 checks, including physical, runtime and checkpoint checks.
It reports runtime cursor 545626 and control-journal sequence 935612. This is
physical/installed verification, not complete independent corpus readback or
proof that every uncertain intent committed. The original estate is preserved
under the owned Linux guest's `desktop-integrated-files-9957ed39` directory;
credentials and private estate files are not published.

| Retained artifact | SHA-256 |
|---|---|
| result.json | `7e0969f369a85fce430f49b6064f7afd0a6d0e2834983358d840f14a25f0711f` |
| launch.json | `fd0745354df37aa341d0bcc2da0eade17b935ced4a407e7caef6ab31e663bf5c` |
| progress.jsonl | `bed905eaf558cfe01ed33170ff60afd18817c9be31b3c98c445092006548933f` |
| post-failure-verify.stdout.json | `86be8789ad3473d0c51cbcb2f651707724efdbc97fe8c921ffda77dde9040cf4` |

The follow-up repair belongs to the existing LSM I/O context: bound idle/open
segment ownership, retain active-read leases, and check native source identity
when reopening through anchored paths. Existing manifests/read-view leases must
continue governing reclamation. The
[RocksDB tuning guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)
documents table-cache descriptor eviction as a reference behavior; no upstream
code is copied. Raising the OS limit or silently selecting another I/O backend
is not the proposed repair.

Automatic approval review rejected the command to publish the new bounded-handle
branch and create its draft PR, returning only `blocked by policy`. Neither
publication ran. The local reviewable brief and opening branch are retained;
no handle-pool implementation or qualification is claimed in this record.

This evidence update was added to the existing PR briefs before editing.
Read the complete native result/launch records, relevant progress summary,
service limit receipt, verifier checks, canonical error/digest formatters and
segment source/open/read ownership. Exact authored scope is this record and
owner links, plus generated navigation/inventory. No runtime effect, resource
increase, source rewrite or gate completion is performed by the evidence update.
Checklist: original identities retained; error independently attributed;
verification scope distinguished from corpus acceptance; failures and limits
preserved; final documentation/projection checks required before commit.
