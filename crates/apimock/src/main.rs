//! App entry point for the `apimock` executable.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some(env_args) = apimock::args::EnvArgs::default()? else {
        // --init was supplied (or similar): work already done, nothing to run
        return Ok(());
    };
    let app = apimock::new(&env_args).await?;
    app.server.start().await;
    Ok(())
}
