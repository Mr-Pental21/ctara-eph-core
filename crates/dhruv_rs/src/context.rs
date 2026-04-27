use dhruv_config::ConfigResolver;
use dhruv_core::{Engine, EngineConfig, LoadedSpkInfo, SpkReplaceReport};
use dhruv_time::TimeConversionPolicy;
use std::path::PathBuf;

use crate::DhruvError;

/// Explicit context for high-level dhruv_rs operations.
///
/// This replaces global singleton usage and keeps configuration caller-owned.
#[derive(Debug)]
pub struct DhruvContext {
    engine: Engine,
    resolver: Option<ConfigResolver>,
    time_policy: TimeConversionPolicy,
}

impl DhruvContext {
    /// Create a context with an initialized engine and default time policy.
    pub fn new(config: EngineConfig) -> Result<Self, DhruvError> {
        let engine = Engine::new(config)?;
        Ok(Self {
            engine,
            resolver: None,
            time_policy: TimeConversionPolicy::default(),
        })
    }

    /// Create a context with resolver-backed layered configuration support.
    pub fn with_resolver(
        config: EngineConfig,
        resolver: ConfigResolver,
    ) -> Result<Self, DhruvError> {
        let engine = Engine::new(config)?;
        Ok(Self {
            engine,
            resolver: Some(resolver),
            time_policy: TimeConversionPolicy::default(),
        })
    }

    /// Borrow the loaded engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Replace the active SPK kernel set while keeping this context alive.
    pub fn replace_spk_paths(
        &self,
        spk_paths: Vec<PathBuf>,
    ) -> Result<SpkReplaceReport, DhruvError> {
        self.engine.replace_spk_paths(spk_paths).map_err(Into::into)
    }

    /// Read information about the active SPK kernels.
    pub fn spk_infos(&self) -> Vec<LoadedSpkInfo> {
        self.engine.spk_infos()
    }

    /// Borrow the optional config resolver.
    pub fn resolver(&self) -> Option<&ConfigResolver> {
        self.resolver.as_ref()
    }

    /// Replace resolver at runtime.
    pub fn set_resolver(&mut self, resolver: Option<ConfigResolver>) {
        self.resolver = resolver;
    }

    /// Set UTC->TDB conversion policy for this context.
    pub fn set_time_conversion_policy(&mut self, policy: TimeConversionPolicy) {
        self.time_policy = policy;
    }

    /// Read UTC->TDB conversion policy for this context.
    pub fn time_conversion_policy(&self) -> TimeConversionPolicy {
        self.time_policy
    }
}
