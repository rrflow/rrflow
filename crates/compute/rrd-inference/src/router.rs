//! Router model artifacts and verified admission.

use super::*;

pub const EMBEDDING_CONTRACT_VERSION: u16 = 1;
pub(super) const MAX_EMBEDDING_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_EMBEDDING_BATCH_INPUTS: u32 = 1_024;
pub const MAX_EMBEDDING_BATCH_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_EMBEDDING_OUTPUT_VALUES: u64 = 1_073_741_824;

/// Immutable bytes presented for router-model admission.
///
/// `tokenizer` is the canonical tokenizer artifact named by the manifest. A
/// container format may physically embed it in the model file, but an adapter
/// must still expose the exact logical tokenizer bytes so its identity can be
/// verified independently before runtime construction.
#[derive(Debug, Clone, Copy)]
pub struct RouterModelArtifacts<'a> {
    model: &'a [u8],
    tokenizer: &'a [u8],
    runtime: &'a [u8],
    grammar: &'a [u8],
}

impl<'a> RouterModelArtifacts<'a> {
    pub const fn new(
        model: &'a [u8],
        tokenizer: &'a [u8],
        runtime: &'a [u8],
        grammar: &'a [u8],
    ) -> Self {
        Self {
            model,
            tokenizer,
            runtime,
            grammar,
        }
    }

    pub const fn model(self) -> &'a [u8] {
        self.model
    }

    pub const fn tokenizer(self) -> &'a [u8] {
        self.tokenizer
    }

    pub const fn runtime(self) -> &'a [u8] {
        self.runtime
    }

    pub const fn grammar(self) -> &'a [u8] {
        self.grammar
    }
}

/// Opaque proof that declarations and all supplied artifact bytes matched.
///
/// This is load admission only. It neither authorizes a route request nor
/// grants a model permission to dispatch work, read an estate, or mutate it.
#[derive(Debug, Clone, Copy)]
pub struct VerifiedRouterModelAdmission<'a> {
    manifest: &'a RouterModelManifest,
    backend: &'a RouterBackendDescriptor,
    handshake: &'a RouterModelHandshake,
    artifacts: RouterModelArtifacts<'a>,
}

impl<'a> VerifiedRouterModelAdmission<'a> {
    pub const fn manifest(&self) -> &'a RouterModelManifest {
        self.manifest
    }

    pub const fn backend(&self) -> &'a RouterBackendDescriptor {
        self.backend
    }

    pub const fn handshake(&self) -> &'a RouterModelHandshake {
        self.handshake
    }

    pub const fn artifacts(&self) -> RouterModelArtifacts<'a> {
        self.artifacts
    }
}

/// Validate the installed manifest, the runtime's independent declaration,
/// and the exact artifact bytes before any model loader or inference backend
/// receives them.
pub fn verify_router_model_before_load<'a>(
    manifest: &'a RouterModelManifest,
    backend: &'a RouterBackendDescriptor,
    handshake: &'a RouterModelHandshake,
    artifacts: RouterModelArtifacts<'a>,
) -> Result<VerifiedRouterModelAdmission<'a>> {
    handshake
        .validate_for(manifest, backend)
        .map_err(|error| Error::InvalidRuntime {
            reason: format!("router model handshake rejected: {error}"),
        })?;
    validate_router_artifact_bytes("model", artifacts.model, &manifest.model)?;
    validate_router_artifact_bytes("tokenizer", artifacts.tokenizer, &manifest.tokenizer)?;
    validate_router_artifact_bytes("runtime", artifacts.runtime, &manifest.runtime.artifact)?;
    validate_router_artifact_bytes("grammar", artifacts.grammar, &manifest.grammar.artifact)?;
    Ok(VerifiedRouterModelAdmission {
        manifest,
        backend,
        handshake,
        artifacts,
    })
}

/// Invoke a model loader only after pre-load admission succeeds.
///
/// Keeping the loader behind this function gives G-01 one enforceable entry
/// point when executable router adapters are introduced; B-03 does not create
/// a router backend, registry, dispatch loop, or LFG authority.
pub fn load_router_model_after_handshake<'a, T>(
    manifest: &'a RouterModelManifest,
    backend: &'a RouterBackendDescriptor,
    handshake: &'a RouterModelHandshake,
    artifacts: RouterModelArtifacts<'a>,
    load: impl FnOnce(VerifiedRouterModelAdmission<'a>) -> Result<T>,
) -> Result<T> {
    let admission = verify_router_model_before_load(manifest, backend, handshake, artifacts)?;
    load(admission)
}
