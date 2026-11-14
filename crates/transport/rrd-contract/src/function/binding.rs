//! Transaction function bindings and effects.

use super::*;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TransactionMutationKind {
    AssertClaim,
    PutSchema,
    PutRecord,
    PutRelation,
    AppendEvent,
    PutVector,
    AppendSeriesSample,
    PutGeo,
    PublishObjectReference,
    RetireData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "effect", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionFunctionEffect {
    RequireTrue,
    AppendEvent { kind: CanonicalId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TransactionFunctionBinding {
    pub binding_id: CanonicalId,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_sha256: Option<String>,
    pub function_id: CanonicalId,
    pub function_definition_sha256: String,
    pub mutation: TransactionMutationKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<CanonicalId>,
    pub effect: TransactionFunctionEffect,
    pub max_attempts: u8,
}

impl TransactionFunctionBinding {
    pub fn validate(&self, functions: &BTreeMap<CanonicalId, FunctionDefinition>) -> Result<()> {
        validate_revision_lineage(
            self.revision,
            self.predecessor_sha256.as_deref(),
            "transaction function binding",
        )?;
        validate_sha256(
            &self.function_definition_sha256,
            "transaction function definition",
        )?;
        if self.max_attempts != 1 {
            return invalid("v1 transaction function bindings require max_attempts=1");
        }
        if self.kind.is_some()
            && matches!(
                self.mutation,
                TransactionMutationKind::AssertClaim
                    | TransactionMutationKind::PutSchema
                    | TransactionMutationKind::RetireData
            )
        {
            return invalid(
                "transaction function binding kind cannot narrow a mutation without a kind coordinate",
            );
        }
        let function = functions.get(&self.function_id).ok_or_else(|| {
            crate::ContractError(
                "transaction function binding references an unknown function".into(),
            )
        })?;
        if function.sha256() != self.function_definition_sha256 {
            return invalid("transaction function binding does not pin its exact definition");
        }
        if matches!(self.effect, TransactionFunctionEffect::AppendEvent { .. })
            && !function
                .capabilities
                .contains(&FunctionCapability::EmitEvent)
        {
            return invalid(
                "append-event transaction function bindings require the emit_event capability",
            );
        }
        Ok(())
    }

    pub fn sha256(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("transaction function binding serializes"))
    }
}
