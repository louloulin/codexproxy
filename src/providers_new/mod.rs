//! New Provider Architecture
//! 
//! This module contains the new provider architecture with:
//! - ProviderRegistry for dynamic routing
//! - Extended LLMProvider trait with preprocessing capabilities
//! - MiMo provider implementation
//! - Error enhancement system
//! - Generic provider loader

mod registry;
mod extended_provider;
mod mimo;
mod error_enhancer;
mod generic_provider;

pub use registry::ProviderRegistry;
pub use extended_provider::{ExtendedLLMProvider, ProviderModel};
pub use mimo::MimoProvider;
pub use error_enhancer::{ErrorEnhancer, EnhancedError};
pub use generic_provider::{
    GenericProviderSpec, GenericProviderModel, GenericFeatures, 
    GenericProviderLoader, WireApi, GenericLoaderError, ProvidersFile,
};
