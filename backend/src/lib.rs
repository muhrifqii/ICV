pub mod controllers;
pub mod entities;
pub use entities::*;
pub mod knowledge;
pub mod service;
pub use service::*;
pub mod utils;
pub use utils::*;

// Export the interface for the smart contract.
ic_cdk::export_candid!();
