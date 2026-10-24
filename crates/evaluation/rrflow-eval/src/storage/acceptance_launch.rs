//! Outer launch/cleanup evidence is distinct from the inner workload outcome.
use super::super::sustained::read_bounded;
use super::super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageLaunchReceipt {
    pub format_version: u16,
    pub campaign_sha256: Digest,
    pub status: CampaignStatus,
    pub exit_code: Option<i64>,
    pub cleanup_confirmed: bool,
    pub allocation: Option<NativeAllocationReceipt>,
    pub elapsed_ms: u64,
    pub reason: Option<Id>,
}
impl StorageLaunchReceipt {
    pub fn validate(&self, campaign: &StorageCampaign) -> Result<()> {
        require(
            self.format_version == 1
                && self.campaign_sha256 == campaign.sha256()?
                && self.elapsed_ms <= MAX_EXACT_INTEGER,
            "launch identity or elapsed time differs",
        )?;
        if let Some(allocation) = &self.allocation {
            allocation.validate(campaign)?;
        }
        require(
            self.exit_code
                .is_none_or(|code| (-2147483648..=4294967295).contains(&code)),
            "launch exit code outside native domain",
        )?;
        if self.status == CampaignStatus::Completed {
            require(
                self.exit_code == Some(0)
                    && self.cleanup_confirmed
                    && self.allocation.is_some()
                    && self.reason.is_none(),
                "completed launch lacks successful owned cleanup",
            )?;
        } else {
            require(self.reason.is_some(), "incomplete launch requires a reason")?;
        }
        if self.status == CampaignStatus::NotRun {
            require(
                self.exit_code.is_none(),
                "not-run launch cannot claim a process exit",
            )?;
        }
        Ok(())
    }
}
fn check_samples(samples: &[ResourceSample], allocation: &Digest) -> Result<()> {
    require(!samples.is_empty(), "launch membership samples absent")?;
    for sample in samples {
        require(
            sample
                .allocation_receipt
                .as_ref()
                .ok_or("launch allocation sample absent")?
                .sha256()?
                == *allocation,
            "raw resource sample belongs to another native launch",
        )?;
    }
    Ok(())
}
fn check_segments(
    root: &Path,
    campaign: &StorageCampaign,
    result: &CampaignResult,
    allocation: &Digest,
) -> Result<()> {
    require(
        result.campaign_sha256 == campaign.sha256()?
            && result.segments > 0
            && result.segments <= campaign.evidence.segments.get(),
        "launch segment count/identity differs",
    )?;
    let mut previous = None;
    for index in 0..result.segments {
        let segment: SustainedSegment = decode(&read_bounded(
            &root.join(format!("segment-{index:020}.json")),
            campaign.evidence.segment_bytes.get(),
        )?)?;
        require(
            segment.index == index
                && segment.previous_sha256 == previous
                && segment.campaign_sha256 == result.campaign_sha256,
            "launch resource source chain changed",
        )?;
        previous = Some(digest("rrflow-storage-segment-v1", &segment)?);
        check_samples(&segment.resources, allocation)?;
    }
    require(
        previous == result.last_segment_sha256,
        "launch resource source differs from verified result",
    )
}
pub(super) fn verify(
    root: &Path,
    campaign: &StorageCampaign,
    launch: &StorageLaunchReceipt,
    expected: Option<&Digest>,
) -> Result<bool> {
    launch.validate(campaign)?;
    if launch.status != CampaignStatus::Completed {
        return Ok(false);
    }
    let expected = expected.ok_or("completed launch omitted inner result")?;
    let allocation = launch
        .allocation
        .as_ref()
        .ok_or("launch allocation absent")?
        .sha256()?;
    if campaign.allocation.estates.get() == 1 {
        let result: CampaignResult = decode(&read_bounded(
            &root.join("result.json"),
            MAX_DOCUMENT_BYTES,
        )?)?;
        require(
            digest("rrflow-storage-campaign-result-v1", &result)? == *expected,
            "launch inner result changed after verification",
        )?;
        check_segments(root, campaign, &result, &allocation)?;
    } else {
        let result: FleetResult = decode(&read_bounded(
            &root.join("fleet-result.json"),
            MAX_DOCUMENT_BYTES,
        )?)?;
        require(
            digest("rrflow-storage-fleet-result-v1", &result)? == *expected
                && result.trials.len() <= 96,
            "launch fleet result changed after verification",
        )?;
        for (index, expected_trial) in result.trials.iter().enumerate() {
            let trial: FleetTrialResult = decode(&read_bounded(
                &root.join(format!("trial-{index:04}.json")),
                MAX_DOCUMENT_BYTES,
            )?)?;
            require(
                digest("rrflow-fleet-trial-v1", &trial)? == *expected_trial
                    && trial.child_results.len() <= 10,
                "launch fleet trial changed after verification",
            )?;
            for boundary in &trial.boundaries {
                check_samples(std::slice::from_ref(&boundary.sample), &allocation)?;
            }
            for (estate, expected_child) in trial.child_results.iter().enumerate() {
                let child = fleet_child(
                    campaign,
                    trial.point,
                    trial.seed,
                    trial.trial,
                    estate as u64,
                )?;
                let directory = fleet_member_directory(root, index as u64, estate as u64);
                let result: CampaignResult = decode(&read_bounded(
                    &directory.join("result.json"),
                    MAX_DOCUMENT_BYTES,
                )?)?;
                require(
                    digest("rrflow-fleet-child-result-v1", &result)? == *expected_child,
                    "launch fleet child changed after verification",
                )?;
                check_segments(&directory, &child, &result, &allocation)?;
            }
        }
    }
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inner_success_cannot_hide_failed_cleanup_or_changed_launch() {
        let campaign = super::super::super::sustained::tests::fixture().0;
        let allocation = NativeAllocationReceipt {
            format_version: 1,
            kind: NativeAllocationKind::CgroupV2,
            nonce: "a".repeat(64).try_into().unwrap(),
            harness_sha256: "b".repeat(64).try_into().unwrap(),
            engine_bytes: campaign.allocation.engine_bytes,
            host_bytes: campaign.allocation.host_bytes,
            vcpus: campaign.allocation.vcpus,
            host_logical_cpus: campaign.allocation.vcpus,
        };
        let mut launch = StorageLaunchReceipt {
            format_version: 1,
            campaign_sha256: campaign.sha256().unwrap(),
            status: CampaignStatus::Completed,
            exit_code: Some(0),
            cleanup_confirmed: true,
            allocation: Some(allocation.clone()),
            elapsed_ms: 1,
            reason: None,
        };
        launch.validate(&campaign).unwrap();
        launch.cleanup_confirmed = false;
        assert!(launch.validate(&campaign).is_err());
        launch.status = CampaignStatus::Failed;
        launch.reason = Some("native-cleanup-unconfirmed".to_owned().try_into().unwrap());
        assert!(!verify(Path::new("unused"), &campaign, &launch, None).unwrap());
        let mut sample = super::super::super::sustained::tests::fixture().1.resources[0].clone();
        sample.allocation_receipt = Some(allocation.clone());
        check_samples(&[sample.clone()], &allocation.sha256().unwrap()).unwrap();
        sample.allocation_receipt.as_mut().unwrap().nonce = "c".repeat(64).try_into().unwrap();
        assert!(check_samples(&[sample], &allocation.sha256().unwrap()).is_err());
        // A correctly rehashed later segment still cannot substitute another
        // native process allocation. Test the whole resource-chain pass.
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().join("run");
        let writer = CampaignWriter::create(&directory, campaign.clone()).unwrap();
        let mut result = writer.finish(CampaignStatus::Failed, vec![]).unwrap();
        let mut segment = super::super::super::sustained::tests::fixture().1;
        segment.resources[0].allocation_receipt = Some(allocation.clone());
        segment.index = 0;
        segment.previous_sha256 = None;
        std::fs::write(
            directory.join("segment-00000000000000000000.json"),
            serde_json::to_vec(&segment).unwrap(),
        )
        .unwrap();
        segment.index = 1;
        segment.previous_sha256 = Some(
            digest(
                "rrflow-storage-segment-v1",
                &decode::<SustainedSegment>(
                    &std::fs::read(directory.join("segment-00000000000000000000.json")).unwrap(),
                )
                .unwrap(),
            )
            .unwrap(),
        );
        let second = directory.join("segment-00000000000000000001.json");
        std::fs::write(&second, serde_json::to_vec(&segment).unwrap()).unwrap();
        result.segments = 2;
        result.last_segment_sha256 = Some(digest("rrflow-storage-segment-v1", &segment).unwrap());
        check_segments(
            &directory,
            &campaign,
            &result,
            &allocation.sha256().unwrap(),
        )
        .unwrap();
        segment.resources[0]
            .allocation_receipt
            .as_mut()
            .unwrap()
            .nonce = "c".repeat(64).try_into().unwrap();
        result.last_segment_sha256 = Some(digest("rrflow-storage-segment-v1", &segment).unwrap());
        std::fs::write(&second, serde_json::to_vec(&segment).unwrap()).unwrap();
        assert!(check_segments(
            &directory,
            &campaign,
            &result,
            &allocation.sha256().unwrap()
        )
        .is_err());
    }
}
