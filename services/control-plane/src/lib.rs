extern crate self as sqlx;

pub(crate) use sqlx_core::query::query;
pub(crate) use sqlx_core::query_scalar::query_scalar;
pub(crate) use sqlx_core::row::Row;
pub(crate) use sqlx_core::transaction::Transaction;
pub(crate) use sqlx_postgres::PgConnection;

pub mod admin_auth;
pub mod app;
pub mod config;
pub mod cost;
pub mod crypto;
pub mod edition_migration;
pub mod import_jobs;
pub mod oauth;
pub mod outbound_proxy_runtime;
pub mod security;
pub mod single_binary;
pub mod store;
pub mod tenant;
#[cfg(test)]
pub(crate) mod test_support;
pub mod upstream_api;
pub mod upstream_error_learning;
pub mod usage;
