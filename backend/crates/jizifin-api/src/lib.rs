//! Jizifin HTTP API library and route table.

pub mod auth;
pub mod config;
pub mod error;
pub mod events;
pub mod health;
pub mod routes;
pub mod scheduler;
pub mod state;

pub use routes::router;
pub use state::AppState;
