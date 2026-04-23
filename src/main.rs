//! App entry point for the `apimock` executable.
//!
//! # Why `anyhow::Result` here
//!
//! Internal code uses the typed `AppError` from `core::error`. At the
//! process boundary we only care about printing a single human-readable
//! line and exiting non-zero, so we flatten into `anyhow::Result` — this
//! gives a free `Display` via `?` in `main` without any ceremony.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some(env_args) = apimock::core::args::EnvArgs::default()? else {
        // --init was supplied (or similar): work already done, nothing to run
        return Ok(());
    };
    let app = apimock::new(&env_args).await?;
    app.server.start().await;
    Ok(())
}
