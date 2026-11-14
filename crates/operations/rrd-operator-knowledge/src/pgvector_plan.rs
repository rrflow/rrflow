//! pgvector SQL plans and relations, and shared validation.

use super::*;

/// Safe, parameterized pgvector query shape. Identifiers are quoted from a
/// validated deployment binding; query vector, tenant, and limit remain `$1`,
/// `$2`, and `$3` parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgvectorSqlPlan {
    pub settings: Vec<(String, String)>,
    pub explain_sql: String,
    pub search_sql: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PgvectorRelation {
    pub schema: String,
    pub relation: String,
    pub id_column: String,
    pub subject_column: String,
    pub vector_column: String,
    pub tenant_column: String,
}

impl PgvectorRelation {
    pub fn validate(&self) -> Result<()> {
        for (kind, value) in [
            ("pgvector schema", &self.schema),
            ("pgvector relation", &self.relation),
            ("pgvector id column", &self.id_column),
            ("pgvector subject column", &self.subject_column),
            ("pgvector vector column", &self.vector_column),
            ("pgvector tenant column", &self.tenant_column),
        ] {
            validate_pg_identifier(kind, value)?;
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        content_digest(b"rrflow-pgvector-relation-v1\0", self)
    }

    pub fn build_search(
        &self,
        metric: ScoreMetric,
        controls: &OperatorSearchControls,
    ) -> Result<PgvectorSqlPlan> {
        self.validate()?;
        controls.validate()?;
        if controls.requested_path == OperatorAccessPath::IvfFlat
            && metric == ScoreMetric::Manhattan
        {
            return invalid("pgvector IVFFlat does not support Manhattan distance");
        }
        let operator = match metric {
            ScoreMetric::Euclidean => "<->",
            ScoreMetric::Dot => "<#>",
            ScoreMetric::Cosine => "<=>",
            ScoreMetric::Manhattan => "<+>",
        };
        let schema = quote_identifier(&self.schema);
        let relation = quote_identifier(&self.relation);
        let id = quote_identifier(&self.id_column);
        let subject = quote_identifier(&self.subject_column);
        let vector = quote_identifier(&self.vector_column);
        let tenant = quote_identifier(&self.tenant_column);
        let score = match metric {
            ScoreMetric::Dot => "distance * -1.0",
            ScoreMetric::Cosine => "1.0 - distance",
            ScoreMetric::Euclidean | ScoreMetric::Manhattan => "distance * -1.0",
        };
        let inner = format!(
            "SELECT {id}::text AS external_id, {subject}::text AS subject_id, {vector} {operator} $1::text::vector AS distance FROM {schema}.{relation} WHERE {tenant}::text = $2 ORDER BY {vector} {operator} $1::text::vector LIMIT $3"
        );
        let search_sql = format!(
            "SELECT external_id, subject_id, {score} AS score FROM ({inner}) AS rrd_ranked ORDER BY distance ASC, external_id ASC"
        );
        let mut settings = Vec::new();
        match controls.requested_path {
            OperatorAccessPath::Exact => {
                settings.push(("enable_indexscan".into(), "off".into()));
                settings.push(("enable_bitmapscan".into(), "off".into()));
            }
            OperatorAccessPath::Hnsw => {
                settings.push(("enable_seqscan".into(), "off".into()));
                push_setting(&mut settings, "hnsw.ef_search", controls.hnsw_ef_search);
                push_setting(
                    &mut settings,
                    "hnsw.max_scan_tuples",
                    controls.hnsw_max_scan_tuples,
                );
                settings.push((
                    "hnsw.iterative_scan".into(),
                    iterative_name(controls.iterative_scan).into(),
                ));
            }
            OperatorAccessPath::IvfFlat => {
                settings.push(("enable_seqscan".into(), "off".into()));
                push_setting(&mut settings, "ivfflat.probes", controls.ivfflat_probes);
                push_setting(
                    &mut settings,
                    "ivfflat.max_probes",
                    controls.ivfflat_max_probes,
                );
                settings.push((
                    "ivfflat.iterative_scan".into(),
                    iterative_name(controls.iterative_scan).into(),
                ));
            }
        }
        Ok(PgvectorSqlPlan {
            settings,
            explain_sql: format!("EXPLAIN (FORMAT JSON) {search_sql}"),
            search_sql,
        })
    }
}

pub(super) fn iterative_name(mode: IterativeScanMode) -> &'static str {
    match mode {
        IterativeScanMode::Off => "off",
        IterativeScanMode::StrictOrder => "strict_order",
        IterativeScanMode::RelaxedOrder => "relaxed_order",
    }
}

pub(super) fn vector_kind(query: &rrd_vector::VectorQuery) -> OperatorVectorKind {
    match query {
        rrd_vector::VectorQuery::Dense { .. } => OperatorVectorKind::Dense,
        rrd_vector::VectorQuery::Sparse { .. } => OperatorVectorKind::Sparse,
        rrd_vector::VectorQuery::MultiDense { .. } => OperatorVectorKind::MultiDense,
    }
}

pub(super) fn push_setting(settings: &mut Vec<(String, String)>, name: &str, value: Option<u32>) {
    if let Some(value) = value {
        settings.push((name.into(), value.to_string()));
    }
}

pub(super) fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub(super) fn validate_pg_identifier(kind: &'static str, value: &str) -> Result<()> {
    validate_name(kind, value)?;
    if value.len() > 63 {
        return invalid(format!(
            "{kind} exceeds PostgreSQL's 63-byte identifier limit"
        ));
    }
    Ok(())
}

pub(super) fn bounded_positive(kind: &'static str, value: Option<u32>, maximum: u32) -> Result<()> {
    if value.is_some_and(|value| value == 0 || value > maximum) {
        return invalid(format!("{kind} must be in 1..={maximum}"));
    }
    Ok(())
}

pub(super) fn content_digest<T: Serialize>(domain: &[u8], value: &T) -> Result<String> {
    let encoded = serde_json::to_vec(value).map_err(|error| Error::InvalidRuntime {
        reason: format!("operator contract cannot be encoded: {error}"),
    })?;
    let mut bytes = domain.to_vec();
    bytes.extend_from_slice(&encoded);
    Ok(digest::sha256_hex(&bytes))
}

pub(super) fn validate_version(version: u16) -> Result<()> {
    if version != OPERATOR_KNOWLEDGE_CONTRACT_VERSION {
        return invalid("unsupported operator-knowledge contract version");
    }
    Ok(())
}

pub(super) fn validate_digest(kind: &'static str, value: &str) -> Result<()> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return invalid(format!("{kind} must be lowercase SHA-256 hex"));
    }
    Ok(())
}

pub(super) fn validate_name(kind: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > MAX_NAME_BYTES || value.as_bytes().contains(&0) {
        return invalid(format!(
            "{kind} must be non-empty, NUL-free, and at most {MAX_NAME_BYTES} bytes"
        ));
    }
    Ok(())
}

pub(super) fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidRuntime {
        reason: reason.into(),
    })
}
