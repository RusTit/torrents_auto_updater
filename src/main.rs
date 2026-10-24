extern crate dotenv;

use dotenv::dotenv;
use std::env;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};
use transmission_rpc::TransClient;
use transmission_rpc::types::{
    BasicAuth, Result, RpcResponse, SessionGet, Torrent, TorrentGetField, Torrents,
};

fn create_client() -> std::result::Result<TransClient, Box<dyn std::error::Error + Send + Sync>> {
    let url = env::var("TRANSMISSION_URL")
        .unwrap_or("http://localhost:9091/transmission/rpc".to_string());
    // Replace with your Transmission RPC URL (e.g., http://localhost:9091/transmission/rpc)
    let url = url.parse()?;
    let user = env::var("TRANSMISSION_LOGIN").unwrap_or_default();
    let password = env::var("TRANSMISSION_PASSWORD").unwrap_or_default();

    if user.is_empty() && password.is_empty() {
        return Ok(TransClient::new(url));
    }

    // Optional basic authentication if enabled on your Transmission daemon
    let basic_auth = BasicAuth { user, password };

    Ok(TransClient::with_auth(url, basic_auth))
}

async fn transmission_torrents_iteration()
-> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = create_client()?;

    // Call session_get() as a test
    let response: Result<RpcResponse<SessionGet>> = client.session_get().await;

    match response {
        Ok(res) => debug!("Connected successfully! Response: {:?}", res),
        Err(e) => warn!("Failed to connect: {:?}", e),
    }

    let response_torrents: RpcResponse<Torrents<Torrent>> = client
        .torrent_get(
            Some(vec![
                TorrentGetField::Id,
                TorrentGetField::HashString,
                TorrentGetField::Name,
                TorrentGetField::DownloadDir,
                TorrentGetField::Error,
                TorrentGetField::ErrorString,
                TorrentGetField::Comment,
            ]),
            None,
        )
        .await?;

    let torrents_with_error: Vec<&Torrent> = response_torrents
        .arguments
        .torrents
        .iter()
        .filter(|it| match it.error {
            None => return false,
            Some(err) => match err {
                transmission_rpc::types::ErrorType::Ok => false,
                transmission_rpc::types::ErrorType::LocalError => true,
                transmission_rpc::types::ErrorType::TrackerWarning => true,
                transmission_rpc::types::ErrorType::TrackerError => true,
            },
        })
        .collect();

    info!(
        "Transmission: matched {} of total {}",
        torrents_with_error.len(),
        response_torrents.arguments.torrents.len()
    );

    Ok(())
}

async fn transmission_torrent_task() {
    loop {
        let result = transmission_torrents_iteration().await;
        match result {
            Ok(()) => debug!("Success"),
            Err(_) => warn!("Error"),
        }
        sleep(Duration::from_secs(60)).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    //dotenv().ok();
    match dotenv() {
        Ok(path) => debug!("Loaded .env from: {}", path.display()),
        Err(e) => warn!("Failed to load .env: {}", e),
    }

    let ((),) = tokio::join!(transmission_torrent_task());

    Ok(())
}
