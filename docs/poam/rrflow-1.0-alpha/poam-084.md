# POAM-084 — the Windows offline qualification sometimes cannot create its disposable account

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-084`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

The nightly full matrix of 2026-11-14 failed on `main` at `1b283d409` ([run 43593938111](https://github.com/rrflow/rrflow-development/actions/runs/43593938111)), job `engram process (windows-latest)`, in the step that qualifies the staged binary without source or outbound access:

```text
File "scripts\ci\windows_offline.py", line 152, in principal
RuntimeError: create qualification principal failed with Windows status 2245
```

Read from the run's log on 2026-11-14. Status 2245 is `NERR_PasswordTooShort`, which Windows also returns when a password fails the local password policy. `windows_offline.py:150` builds the password as `"Aa1!"` plus `secrets.token_urlsafe(32)`, 47 characters with all four character classes.

Every macOS job in that run passed. The three nightlies before it passed this step. The cause is not known: it is either a property of some generated passwords or of the runner image that night.

## Impact

The nightly matrix is the only place native Windows qualification runs. A failure here that is not about the product turns the main alarm on for nothing and, repeated, teaches people to ignore it.

## Owning gates

D-01 owns the installed lifecycle and its native offline qualification.

Roadmap owner: [Gate D](../../roadmap/rrflow-1.0/gate-d.md).

## Closure evidence

Closed when the cause is identified from a retained failure (the generated password's shape, never its value, and the host's password policy) and either the generator can no longer produce a rejected password or the step reports a host-policy refusal as that, and the nightly passes this step on repeated runs.
