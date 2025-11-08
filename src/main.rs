/// app entry point on executable
#[tokio::main]
async fn main() {
    let env_args = match apimock::core::args::EnvArgs::default() {
        Some(x) => x,
        None => return (),
    };
    let app = apimock::new(&env_args).await;
    app.server.start().await
}
