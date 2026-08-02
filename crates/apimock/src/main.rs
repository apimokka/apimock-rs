//! App entry point for the `apimock` executable.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some(env_args) = apimock::args::EnvArgs::default()? else {
        // --init was supplied (or similar): work already done, nothing to run
        return Ok(());
    };
    let app = apimock::App::new(&env_args, None, true).await?;
    app.server.start().await;
    Ok(())
}
