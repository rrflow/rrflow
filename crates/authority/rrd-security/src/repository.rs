//! The security repository.

use super::*;

pub struct SecurityRepository<'a, E> {
    engine: &'a E,
    instance: CanonicalId,
}

impl<'a, E: StorageEngine> SecurityRepository<'a, E> {
    pub fn new(engine: &'a E, instance: CanonicalId) -> Self {
        Self { engine, instance }
    }

    pub fn load(&self) -> Result<Option<SecurityState>> {
        self.engine
            .control()
            .get(&security_key(&self.instance))?
            .map(|bytes| decode_state(&bytes))
            .transpose()
    }

    /// Exact current authority bytes for an engine-owned atomic effect guard.
    /// This is a storage identity, not a grant or a public credential response.
    pub fn authority_snapshot(&self) -> Result<Option<(String, Vec<u8>)>> {
        let key = security_key(&self.instance);
        let Some(bytes) = self.engine.control().get(&key)? else {
            return Ok(None);
        };
        decode_state(&bytes)?;
        Ok(Some((key, bytes)))
    }

    pub fn initialize(
        &self,
        state: SecurityState,
        at: u64,
        actor: &str,
        request_id: &str,
        operation_id: &str,
    ) -> Result<()> {
        if self.load()?.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        let transitions = prepare_security_initialization(
            &self.instance,
            &state,
            at,
            actor,
            request_id,
            operation_id,
        )?;
        self.engine.control().commit_batch(&transitions)?;
        Ok(())
    }

    /// Replaces the complete security authority with one compare-and-swap
    /// transition. Exact replay is a no-op; revision gaps, stale writers, and
    /// payload substitution fail closed.
    pub fn replace(
        &self,
        expected_revision: u64,
        replacement: SecurityState,
        at: u64,
        actor: &str,
        request_id: &str,
        operation_id: &str,
    ) -> Result<()> {
        replacement.validate()?;
        if replacement.revision != expected_revision.saturating_add(1) {
            return Err(Error::Invalid(
                "security replacement must advance exactly one revision".into(),
            ));
        }
        let key = security_key(&self.instance);
        let current_bytes = self
            .engine
            .control()
            .get(&key)?
            .ok_or(Error::NotInitialized)?;
        let current = decode_state(&current_bytes)?;
        if current == replacement {
            return Ok(());
        }
        if current.revision != expected_revision {
            return Err(Error::IdempotencyConflict);
        }
        let replacement_bytes = serde_json::to_vec(&replacement).map_err(json_error)?;
        let transition = ControlTransition {
            key,
            expected: Some(current_bytes),
            replacement: Some(replacement_bytes.clone()),
            at,
            actor: actor.into(),
            action: "security.replaced".into(),
            request_id: request_id.into(),
            operation_id: operation_id.into(),
        };
        let audit = self.administrative_audit(
            at,
            request_id,
            operation_id,
            &digest::sha256_hex(&replacement_bytes),
            "security-authority-replaced",
        )?;
        self.commit_with_audit(&[transition], &audit)
    }

    pub fn authenticate_and_authorize(
        &self,
        principal_id: &CanonicalId,
        credential: &[u8],
        action: Action,
        resource: &ResourcePath,
        at: u64,
    ) -> Result<Authorization> {
        self.authenticate_principal(principal_id, credential)?;
        let state = self.load()?.ok_or(Error::NotInitialized)?;
        compile_authorization(&state, principal_id, action, resource, at)
    }

    /// Authenticates identity without granting an action. This split allows
    /// callers to retain a proven principal on a subsequent policy denial
    /// while never attributing a failed credential attempt to that principal.
    pub fn authenticate_principal(
        &self,
        principal_id: &CanonicalId,
        credential: &[u8],
    ) -> Result<()> {
        let state = self.load()?.ok_or(Error::NotInitialized)?;
        let principal = state
            .principals
            .get(principal_id)
            .ok_or(Error::PrincipalNotFound)?;
        let supplied = digest::sha256_hex(credential);
        if !constant_time_equal(supplied.as_bytes(), principal.credential_sha256.as_bytes()) {
            return Err(Error::Unauthenticated);
        }
        Ok(())
    }

    /// Re-evaluates current policy for a principal whose credential was
    /// authenticated when its short-lived RRD session was created.
    pub fn authorize_principal(
        &self,
        principal_id: &CanonicalId,
        action: Action,
        resource: &ResourcePath,
        at: u64,
    ) -> Result<Authorization> {
        let state = self.load()?.ok_or(Error::NotInitialized)?;
        compile_authorization(&state, principal_id, action, resource, at)
    }

    /// Resolves an identity assertion only after a transport adapter has
    /// cryptographically verified the third-party token. The exact
    /// issuer/subject/audience tuple is persisted here; unbound assertions are
    /// never treated as application principals.
    pub fn resolve_verified_identity(
        &self,
        issuer: &str,
        subject: &str,
        audience: &str,
        at: u64,
    ) -> Result<CanonicalId> {
        let state = self.load()?.ok_or(Error::NotInitialized)?;
        let binding = state
            .identity_bindings
            .values()
            .find(|binding| {
                !binding.disabled
                    && binding.issuer == issuer
                    && binding.subject == subject
                    && binding.audience == audience
            })
            .ok_or(Error::Unauthenticated)?;
        let principal = state
            .principals
            .get(&binding.principal_id)
            .ok_or(Error::PrincipalNotFound)?;
        require_active_principal(principal, at)?;
        Ok(principal.id.clone())
    }

    pub fn issue_jwt(
        &self,
        principal_id: &CanonicalId,
        credential: &[u8],
        signing_key: &[u8],
        request: &JwtIssueRequest,
    ) -> Result<JwtCredential> {
        self.authenticate_principal(principal_id, credential)?;
        let state = self.load()?.ok_or(Error::NotInitialized)?;
        let principal = state
            .principals
            .get(principal_id)
            .ok_or(Error::PrincipalNotFound)?;
        require_active_principal(principal, request.issued_at_unix_ms)?;
        let issuer = state
            .jwt_issuers
            .get(&request.issuer_id)
            .ok_or(Error::Unauthenticated)?;
        validate_jwt_window(issuer, request)?;
        if digest::sha256_hex(signing_key) != issuer.signing_key_sha256 {
            return Err(Error::Unauthenticated);
        }
        let header = JwtHeader {
            alg: "HS256".into(),
            typ: "JWT".into(),
            kid: issuer.key_id.clone(),
        };
        let claims = JwtClaims {
            iss: issuer.issuer.clone(),
            sub: principal.id.clone(),
            aud: issuer.audience.clone(),
            iat: request.issued_at_unix_ms,
            nbf: request.not_before_unix_ms,
            exp: request.expires_at_unix_ms,
            jti: request.token_id.clone(),
            policy_revision: state.revision,
            credential_revision: principal.credential_revision,
        };
        let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(json_error)?);
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).map_err(json_error)?);
        let signed = format!("{header}.{claims}");
        let signature = jwt_signature(signing_key, signed.as_bytes())?;
        let token = format!("{signed}.{}", URL_SAFE_NO_PAD.encode(signature));
        if token.len() > MAX_JWT_BYTES {
            return Err(Error::Invalid("issued JWT exceeds its byte bound".into()));
        }
        Ok(JwtCredential {
            token,
            expires_at_unix_ms: request.expires_at_unix_ms,
        })
    }

    /// Validates signature and registered claims against the current
    /// authority. Credential rotation or principal/issuer revocation rejects
    /// an otherwise cryptographically valid older token.
    pub fn authenticate_jwt(
        &self,
        token: &str,
        signing_key: &[u8],
        at: u64,
    ) -> Result<CanonicalId> {
        if token.is_empty() || token.len() > MAX_JWT_BYTES {
            return Err(Error::Unauthenticated);
        }
        let mut segments = token.split('.');
        let header_segment = segments.next().ok_or(Error::Unauthenticated)?;
        let claims_segment = segments.next().ok_or(Error::Unauthenticated)?;
        let signature_segment = segments.next().ok_or(Error::Unauthenticated)?;
        if segments.next().is_some() {
            return Err(Error::Unauthenticated);
        }
        let header: JwtHeader = decode_jwt_segment(header_segment)?;
        let claims: JwtClaims = decode_jwt_segment(claims_segment)?;
        if header.alg != "HS256" || header.typ != "JWT" {
            return Err(Error::Unauthenticated);
        }
        let state = self.load()?.ok_or(Error::NotInitialized)?;
        let issuer = state
            .jwt_issuers
            .values()
            .find(|issuer| issuer.key_id == header.kid)
            .ok_or(Error::Unauthenticated)?;
        if issuer.disabled
            || issuer.issuer != claims.iss
            || issuer.audience != claims.aud
            || at < issuer.not_before_unix_ms
            || at >= issuer.expires_at_unix_ms
            || digest::sha256_hex(signing_key) != issuer.signing_key_sha256
        {
            return Err(Error::Unauthenticated);
        }
        let signature = URL_SAFE_NO_PAD
            .decode(signature_segment)
            .map_err(|_| Error::Unauthenticated)?;
        let signed = format!("{header_segment}.{claims_segment}");
        verify_jwt_signature(signing_key, signed.as_bytes(), &signature)?;
        if claims.iat == 0
            || claims.nbf < claims.iat
            || claims.exp <= claims.nbf
            || claims.exp.saturating_sub(claims.iat) > MAX_JWT_LIFETIME_MS
            || at < claims.nbf
            || at >= claims.exp
            || claims.policy_revision > state.revision
        {
            return Err(Error::Unauthenticated);
        }
        let principal = state
            .principals
            .get(&claims.sub)
            .ok_or(Error::Unauthenticated)?;
        require_active_principal(principal, at).map_err(|_| Error::Unauthenticated)?;
        if claims.credential_revision != principal.credential_revision {
            return Err(Error::Unauthenticated);
        }
        Ok(principal.id.clone())
    }

    pub fn is_initialized(&self) -> Result<bool> {
        Ok(self.load()?.is_some())
    }

    pub fn append_audit(&self, event: &AuditEvent) -> Result<()> {
        self.commit_with_audit(&[], event)
    }

    pub fn audit_since(&self, after: u64, limit: usize) -> Result<AuditJournalPage> {
        if limit == 0 || limit > MAX_AUDIT_PAGE {
            return Err(Error::Invalid("audit page limit is outside bounds".into()));
        }
        let mut cursor = after;
        let mut records = Vec::new();
        let mut reached_control_tail = false;
        while records.len() < limit {
            let entries = self.engine.control().journal_since(cursor, limit)?;
            if entries.is_empty() {
                reached_control_tail = true;
                break;
            }
            let mut consumed = 0;
            for entry in &entries {
                consumed += 1;
                cursor = entry.sequence;
                if entry.action == "security.audit" {
                    records.push((entry.sequence, decode_audit(entry)?));
                    if records.len() == limit {
                        break;
                    }
                }
            }
            if entries.len() < limit && consumed == entries.len() {
                reached_control_tail = true;
                break;
            }
        }
        let durable_head = self
            .engine
            .control()
            .get(&audit_head_key(&self.instance))?
            .as_deref()
            .map(decode_audit_head)
            .transpose()?;
        if let Some(head) = &durable_head {
            let head_record = self
                .engine
                .control()
                .get(&audit_key(&self.instance, &head.audit_id))?
                .ok_or_else(|| Error::Invalid("audit head record is missing".into()))?;
            let head_record = decode_audit_bytes(&head_record)?;
            if head_record.audit_sha256 != head.audit_sha256 {
                return Err(Error::Invalid("audit head digest is invalid".into()));
            }
        } else if !records.is_empty() {
            return Err(Error::Invalid("audit records exist without a head".into()));
        }
        let chain_anchor_sha256 = records
            .first()
            .and_then(|(_, record)| record.previous_audit_sha256.clone());
        let mut expected = chain_anchor_sha256.clone();
        for (_, record) in &records {
            if record.previous_audit_sha256 != expected {
                return Err(Error::Invalid("audit chain continuity is invalid".into()));
            }
            expected = Some(record.audit_sha256.clone());
        }
        if after == 0 && chain_anchor_sha256.is_some() {
            return Err(Error::Invalid("audit chain genesis is invalid".into()));
        }
        if let (Some((_, last)), Some(head)) = (records.last(), durable_head.as_ref()) {
            if last.audit_id == head.audit_id {
                if last.audit_sha256 != head.audit_sha256
                    || (after == 0 && records.len() as u64 != head.record_count)
                {
                    return Err(Error::Invalid("audit head closure is invalid".into()));
                }
            } else if reached_control_tail {
                return Err(Error::Invalid(
                    "audit chain does not reach its durable head".into(),
                ));
            }
        }
        Ok(AuditJournalPage {
            through_sequence: cursor,
            chain_anchor_sha256,
            chain_head_sha256: expected,
            records,
        })
    }

    fn commit_with_audit(
        &self,
        additional: &[ControlTransition],
        event: &AuditEvent,
    ) -> Result<()> {
        event.validate()?;
        let record_key = audit_key(&self.instance, &event.audit_id);
        if let Some(existing) = self.engine.control().get(&record_key)? {
            return exact_audit_replay(&existing, event);
        }
        let head_key = audit_head_key(&self.instance);
        for _ in 0..16 {
            let head_bytes = self.engine.control().get(&head_key)?;
            let head = head_bytes.as_deref().map(decode_audit_head).transpose()?;
            let record = AuditRecord::seal(
                event.clone(),
                head.as_ref().map(|head| head.audit_sha256.clone()),
            )?;
            let next_head = AuditHead {
                audit_id: record.audit_id.clone(),
                audit_sha256: record.audit_sha256.clone(),
                record_count: head.as_ref().map_or(Ok(1), |head| {
                    head.record_count
                        .checked_add(1)
                        .ok_or(Error::Invalid("audit record count overflow".into()))
                })?,
            };
            let record_bytes = serde_json::to_vec(&record).map_err(json_error)?;
            let mut transitions = additional.to_vec();
            transitions.push(ControlTransition {
                key: record_key.clone(),
                expected: None,
                replacement: Some(record_bytes),
                at: event.at_unix_ms,
                actor: event
                    .principal_id
                    .as_ref()
                    .map_or("anonymous", CanonicalId::as_str)
                    .into(),
                action: "security.audit".into(),
                request_id: event.request_id.clone(),
                operation_id: event.operation_id.clone(),
            });
            transitions.push(ControlTransition {
                key: head_key.clone(),
                expected: head_bytes.clone(),
                replacement: Some(serde_json::to_vec(&next_head).map_err(json_error)?),
                at: event.at_unix_ms,
                actor: event
                    .principal_id
                    .as_ref()
                    .map_or("anonymous", CanonicalId::as_str)
                    .into(),
                action: "security.audit.head".into(),
                request_id: event.request_id.clone(),
                operation_id: event.operation_id.clone(),
            });
            match self.engine.control().commit_batch(&transitions) {
                Ok(_) => return Ok(()),
                Err(rrd_store::Error::ControlConflict(key)) if key == head_key => continue,
                Err(rrd_store::Error::ControlConflict(key)) if key == record_key => {
                    let existing =
                        self.engine.control().get(&record_key)?.ok_or_else(|| {
                            Error::Invalid("audit identity conflict vanished".into())
                        })?;
                    return exact_audit_replay(&existing, event);
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(Error::Store(rrd_store::Error::Substrate(
            "audit head contention exceeded its retry bound".into(),
        )))
    }

    fn administrative_audit(
        &self,
        at_unix_ms: u64,
        request_id: &str,
        operation_id: &str,
        request_sha256: &str,
        outcome: &str,
    ) -> Result<AuditEvent> {
        administrative_audit_for_instance(
            &self.instance,
            at_unix_ms,
            request_id,
            operation_id,
            request_sha256,
            outcome,
        )
    }
}
