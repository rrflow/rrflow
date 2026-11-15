use rrd_contract::{CanonicalId, EngramDesiredPhase};
use rrd_engine::{EngramAdminAction, RrdEngine};
use std::path::PathBuf;

#[derive(Debug)]
struct Args {
    action: EngramAdminAction,
    db: PathBuf,
    authority_instance: CanonicalId,
    policy: PathBuf,
    key: PathBuf,
    engram: CanonicalId,
    at: u64,
    request_id: String,
    operation_id: CanonicalId,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("rrd-engram-admin: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args(std::env::args().skip(1))?;
    let result = RrdEngine::administer_engram_store(
        &args.db,
        args.authority_instance,
        &args.policy,
        &args.key,
        args.engram,
        args.at,
        args.request_id,
        args.operation_id,
        args.action,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn parse_args(mut arguments: impl Iterator<Item = String>) -> Result<Args, String> {
    let action_name = arguments.next().ok_or_else(|| usage().to_owned())?;
    let mut db = None;
    let mut authority_instance = None;
    let mut policy = None;
    let mut key = None;
    let mut engram = None;
    let mut at = None;
    let mut request_id = None;
    let mut operation_id = None;
    let mut instance = None;
    let mut idempotency_key = None;
    let mut phase = None;
    let mut deployment = None;
    let mut version = None;
    let mut configuration_sha256 = None;
    let mut label = None;
    let mut max_rpo_ms = None;
    let mut max_rto_ms = None;
    let mut minimum_recovery_points = None;
    let mut retention_ms = None;
    let mut backup_sha256 = None;
    let mut expires_at_unix_ms = None;
    let mut pin_id = None;
    while let Some(argument) = arguments.next() {
        let value = required(&mut arguments, &argument)?;
        match argument.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--authority-instance" => {
                authority_instance = Some(canonical(value, "--authority-instance")?);
            }
            "--policy" => policy = Some(PathBuf::from(value)),
            "--key" => key = Some(PathBuf::from(value)),
            "--engram" => engram = Some(canonical(value, "--engram")?),
            "--at" => at = Some(value.parse().map_err(|_| "--at must be u64")?),
            "--request" => request_id = Some(value),
            "--operation" => operation_id = Some(canonical(value, "--operation")?),
            "--instance" => instance = Some(canonical(value, "--instance")?),
            "--idempotency" => idempotency_key = Some(value),
            "--phase" => phase = Some(parse_phase(&value)?),
            "--deployment" => deployment = Some(canonical(value, "--deployment")?),
            "--version" => version = Some(value),
            "--configuration-sha256" => configuration_sha256 = Some(value),
            "--label" => label = Some(value),
            "--max-rpo-ms" => {
                max_rpo_ms = Some(value.parse().map_err(|_| "--max-rpo-ms must be u64")?)
            }
            "--max-rto-ms" => {
                max_rto_ms = Some(value.parse().map_err(|_| "--max-rto-ms must be u64")?)
            }
            "--minimum-recovery-points" => {
                minimum_recovery_points = Some(
                    value
                        .parse()
                        .map_err(|_| "--minimum-recovery-points must be u16")?,
                )
            }
            "--retention-ms" => {
                retention_ms = Some(value.parse().map_err(|_| "--retention-ms must be u64")?)
            }
            "--backup-sha256" => backup_sha256 = Some(value),
            "--expires-at" => {
                expires_at_unix_ms = Some(value.parse().map_err(|_| "--expires-at must be u64")?)
            }
            "--pin" => pin_id = Some(canonical(value, "--pin")?),
            _ => return Err(format!("unknown option {argument:?}\n{}", usage())),
        }
    }
    let action = match action_name.as_str() {
        "create" => {
            if instance.is_some()
                || idempotency_key.is_some()
                || phase.is_some()
                || deployment.is_some()
                || version.is_some()
                || configuration_sha256.is_some()
                || label.is_some()
                || max_rpo_ms.is_some()
                || max_rto_ms.is_some()
                || minimum_recovery_points.is_some()
                || retention_ms.is_some()
                || backup_sha256.is_some()
                || expires_at_unix_ms.is_some()
                || pin_id.is_some()
            {
                return Err("create does not accept desired-state options".into());
            }
            EngramAdminAction::Create
        }
        "set-desired" => {
            if label.is_some()
                || max_rpo_ms.is_some()
                || max_rto_ms.is_some()
                || minimum_recovery_points.is_some()
                || retention_ms.is_some()
                || backup_sha256.is_some()
                || expires_at_unix_ms.is_some()
                || pin_id.is_some()
            {
                return Err("set-desired received an unrelated option".into());
            }
            EngramAdminAction::SetDesired {
                instance: instance.ok_or("--instance is required")?,
                idempotency_key: idempotency_key.ok_or("--idempotency is required")?,
                phase: phase.ok_or("--phase is required")?,
                deployment: deployment.ok_or("--deployment is required")?,
                version: version.ok_or("--version is required")?,
                configuration_sha256: configuration_sha256
                    .ok_or("--configuration-sha256 is required")?,
            }
        }
        "schedule-backup" => {
            if phase.is_some()
                || deployment.is_some()
                || version.is_some()
                || configuration_sha256.is_some()
                || max_rpo_ms.is_some()
                || max_rto_ms.is_some()
                || minimum_recovery_points.is_some()
                || retention_ms.is_some()
                || backup_sha256.is_some()
                || expires_at_unix_ms.is_some()
                || pin_id.is_some()
            {
                return Err("schedule-backup does not accept desired-state options".into());
            }
            EngramAdminAction::ScheduleBackup {
                instance: instance.ok_or("--instance is required")?,
                idempotency_key: idempotency_key.ok_or("--idempotency is required")?,
                label: label.ok_or("--label is required")?,
            }
        }
        "set-recovery-policy" => {
            if phase.is_some()
                || deployment.is_some()
                || version.is_some()
                || configuration_sha256.is_some()
                || label.is_some()
                || backup_sha256.is_some()
                || expires_at_unix_ms.is_some()
                || pin_id.is_some()
            {
                return Err("set-recovery-policy received an unrelated option".into());
            }
            EngramAdminAction::SetRecoveryPolicy {
                instance: instance.ok_or("--instance is required")?,
                idempotency_key: idempotency_key.ok_or("--idempotency is required")?,
                max_rpo_ms: max_rpo_ms.ok_or("--max-rpo-ms is required")?,
                max_rto_ms: max_rto_ms.ok_or("--max-rto-ms is required")?,
                minimum_recovery_points: minimum_recovery_points
                    .ok_or("--minimum-recovery-points is required")?,
                retention_ms: retention_ms.ok_or("--retention-ms is required")?,
            }
        }
        "pin-recovery-point" => {
            if instance.is_some()
                || phase.is_some()
                || deployment.is_some()
                || version.is_some()
                || configuration_sha256.is_some()
                || label.is_some()
                || max_rpo_ms.is_some()
                || max_rto_ms.is_some()
                || minimum_recovery_points.is_some()
                || retention_ms.is_some()
                || pin_id.is_some()
            {
                return Err("pin-recovery-point received an unrelated option".into());
            }
            EngramAdminAction::PinRecoveryPoint {
                idempotency_key: idempotency_key.ok_or("--idempotency is required")?,
                backup_sha256: backup_sha256.ok_or("--backup-sha256 is required")?,
                expires_at_unix_ms,
            }
        }
        "release-recovery-pin" => {
            if instance.is_some()
                || phase.is_some()
                || deployment.is_some()
                || version.is_some()
                || configuration_sha256.is_some()
                || label.is_some()
                || max_rpo_ms.is_some()
                || max_rto_ms.is_some()
                || minimum_recovery_points.is_some()
                || retention_ms.is_some()
                || backup_sha256.is_some()
                || expires_at_unix_ms.is_some()
            {
                return Err("release-recovery-pin received an unrelated option".into());
            }
            EngramAdminAction::ReleaseRecoveryPin {
                idempotency_key: idempotency_key.ok_or("--idempotency is required")?,
                pin_id: pin_id.ok_or("--pin is required")?,
            }
        }
        _ => return Err(usage().into()),
    };
    Ok(Args {
        action,
        db: db.ok_or("--db is required")?,
        authority_instance: authority_instance.ok_or("--authority-instance is required")?,
        policy: policy.ok_or("--policy is required")?,
        key: key.ok_or("--key is required")?,
        engram: engram.ok_or("--engram is required")?,
        at: at.ok_or("--at is required")?,
        request_id: request_id.ok_or("--request is required")?,
        operation_id: operation_id.ok_or("--operation is required")?,
    })
}

fn required(arguments: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn canonical(value: String, option: &str) -> Result<CanonicalId, String> {
    CanonicalId::new(value).map_err(|error| format!("{option}: {error}"))
}

fn parse_phase(value: &str) -> Result<EngramDesiredPhase, String> {
    match value {
        "running" => Ok(EngramDesiredPhase::Running),
        "stopped" => Ok(EngramDesiredPhase::Stopped),
        "absent" => Ok(EngramDesiredPhase::Absent),
        _ => Err("--phase must be running, stopped, or absent".into()),
    }
}

fn usage() -> &'static str {
    "usage: rrd-engram-admin <create|set-desired|schedule-backup|set-recovery-policy|pin-recovery-point|release-recovery-pin> --db PATH --authority-instance ID --policy PATH --key PATH --engram ID --at UNIX_MS --request ID --operation ID [--instance ID --idempotency KEY] [--phase running|stopped|absent --deployment ID --version VERSION --configuration-sha256 SHA256] [--label LABEL] [--max-rpo-ms MS --max-rto-ms MS --minimum-recovery-points COUNT --retention-ms MS] [--backup-sha256 SHA256 --expires-at UNIX_MS] [--pin ID]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_a_distinct_authority_instance() {
        let error = parse_args(
            [
                "create",
                "--db",
                "/tmp/db",
                "--policy",
                "/tmp/policy",
                "--key",
                "/tmp/key",
                "--engram",
                "engram-a",
                "--at",
                "1",
                "--request",
                "request-a",
                "--operation",
                "operation-a",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap_err();
        assert_eq!(error, "--authority-instance is required");
    }
}
