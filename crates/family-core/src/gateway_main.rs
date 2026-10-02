use std::path::PathBuf;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: family-gateway <storage-directory> [provider-config.json]")?,
    );
    let providers = args.next().map(PathBuf::from);
    let app = family_core::gateway::router(root, providers)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787").await?;
    eprintln!("Family Room encrypted gateway: http://127.0.0.1:8787 (use an HTTPS reverse proxy for remote access)");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
