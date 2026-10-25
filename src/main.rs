extern crate dotenv;

use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};
use transmission_rpc::TransClient;
use transmission_rpc::types::{
    BasicAuth, Result, RpcResponse, SessionGet, Torrent, TorrentGetField, Torrents,
};

mod flaresolverr_proxy;

use flaresolverr_proxy::FlareSolverrProxy;

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

async fn fetch_unregistered_torrents(
    client: &mut TransClient,
) -> std::result::Result<Vec<Torrent>, Box<dyn std::error::Error + Send + Sync>> {
    let response_torrents: RpcResponse<Torrents<Torrent>> = client
        .torrent_get(
            // Some(vec![
            //     TorrentGetField::Id,
            //     TorrentGetField::HashString,
            //     TorrentGetField::Name,
            //     TorrentGetField::DownloadDir,
            //     TorrentGetField::Error,
            //     TorrentGetField::ErrorString,
            //     TorrentGetField::Comment,
            // ]),
            None, None,
        )
        .await?;
    let total_torrents_count = response_torrents.arguments.torrents.len();

    let torrents_with_error: Vec<Torrent> = response_torrents
        .arguments
        .torrents
        .into_iter()
        .filter(|torrent| {
            torrent.tracker_stats.as_ref().is_some_and(|stats| {
                stats.iter().any(|stat| {
                    stat.last_announce_result
                        .to_lowercase()
                        .contains("unregistered")
                })
            })
        })
        .collect();

    info!("{:?}", torrents_with_error);
    info!(
        "Transmission: matched {} of total {}",
        torrents_with_error.len(),
        total_torrents_count
    );
    Ok(torrents_with_error)
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

    let torrents_with_error = fetch_unregistered_torrents(&mut client).await?;

    if torrents_with_error.len() == 0 {
        info!("No torrents with errors");
        return Ok(());
    }

    let fl = FlareSolverrProxy::new(env::var("FLARESOLVERR_URL")?, env::var("PROXY_URL").ok());
    let session_id = fl.get_or_create_session_id().await?;

    for torrent in torrents_with_error.iter() {
        if let Some(comment) = &torrent.comment {
        } else {
            warn!(
                "Torrent {:?} {:?} don't have comment",
                torrent.id,
                torrent.name.as_ref()
            );
        }
    }

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
