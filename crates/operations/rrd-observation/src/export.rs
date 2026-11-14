//! Prepared observation exports.

use super::*;

pub struct PreparedExport<'a> {
    url: String,
    payload: &'a [u8],
    expected_event_id: &'a str,
    policy: &'a ObservationExportPolicy,
    timeout: Duration,
}

impl<'a> PreparedExport<'a> {
    /// Pure preparation is not permission to contact the destination.
    pub fn new(
        binding: &EndpointBinding,
        policy: &'a ObservationExportPolicy,
        payload: &'a [u8],
        expected_event_id: &'a str,
    ) -> Result<Self, &'static str> {
        policy
            .validate_for(std::slice::from_ref(binding))
            .map_err(|_| "invalid observation binding")?;
        if !policy.allow_network || !policy.allow_metadata_export {
            return Err("observation export is disabled");
        }
        if payload.is_empty()
            || payload.len() > MAX_OBSERVATION_EVENT_BYTES
            || payload.len() as u64 > binding.limits.max_input_bytes
        {
            return Err("observation payload exceeds configured limit");
        }
        let EndpointLocator::Url { url } = &binding.endpoint else {
            return Err("observation transport requires a URL binding");
        };
        let uri: ureq::http::Uri = url.parse().map_err(|_| "invalid observation URL")?;
        let host = uri.host().ok_or("observation URL has no host")?;
        let loopback = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
        if uri.query().is_some()
            || url.contains('#')
            || uri.authority().is_some_and(|a| a.as_str().contains('@'))
            || !(uri.scheme_str() == Some("https") || uri.scheme_str() == Some("http") && loopback)
        {
            return Err(
                "observation URL requires TLS or literal loopback without credentials/query",
            );
        }
        Ok(Self {
            url: format!("{}/v1/events", url.trim_end_matches('/')),
            payload,
            expected_event_id,
            policy,
            timeout: Duration::from_millis(binding.limits.timeout_ms),
        })
    }

    /// Call only after the engine commits and authorizes its dispatch fence.
    /// One exchange, no redirects/proxies/retry. In-flight revocation cannot
    /// undo a remote write; the engine must separately validate the returned facts.
    pub fn execute(&self, credential: &str, deadline: Instant) -> TransportObservation {
        use ObservationDeliveryDisposition as D;
        if credential.is_empty()
            || credential.len() > 16_384
            || credential
                .bytes()
                .any(|c| c.is_ascii_whitespace() || c.is_ascii_control())
        {
            return TransportObservation::failure(D::CredentialUnavailable, false);
        }
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return TransportObservation::failure(D::DeadlineExceeded, false);
        };
        let Ok(mut authorization) =
            ureq::http::HeaderValue::from_str(&format!("Bearer {credential}"))
        else {
            return TransportObservation::failure(D::CredentialUnavailable, false);
        };
        authorization.set_sensitive(true);
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .max_idle_connections(0)
            .timeout_global(Some(remaining.min(self.timeout)))
            .build()
            .into();
        let response = agent
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Authorization", authorization)
            .send(self.payload);
        let Ok(mut response) = response else {
            return TransportObservation::failure(D::TransportUnavailable, true);
        };
        let status = response.status().as_u16();
        let mut observation = TransportObservation::failure(D::ReplyRejected, true);
        observation.http_status = Some(status);
        if Instant::now() >= deadline {
            observation.disposition = D::DeadlineExceeded;
            return observation;
        }
        if status != 200 {
            return observation;
        }
        let mut bytes = Vec::new();
        if response
            .body_mut()
            .as_reader()
            .take(self.policy.max_response_bytes + 1)
            .read_to_end(&mut bytes)
            .is_err()
        {
            observation.disposition = D::TransportUnavailable;
            return observation;
        }
        if bytes.len() as u64 > self.policy.max_response_bytes {
            observation.disposition = D::ReplyTooLarge;
            return observation;
        }
        if Instant::now() >= deadline {
            observation.disposition = D::DeadlineExceeded;
            return observation;
        }
        observation.reply_sha256 = Some(sha256_hex(&bytes));
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Acknowledgement {
            devspace: String,
            accepted_ids: Vec<String>,
        }
        if let Ok(reply) = serde_json::from_slice::<Acknowledgement>(&bytes) {
            if reply.devspace == self.policy.expected_devspace
                && reply.accepted_ids == [self.expected_event_id]
            {
                observation.disposition = D::Acknowledged;
            }
        }
        observation
    }
}
