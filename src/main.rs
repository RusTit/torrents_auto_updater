extern crate dotenv;

use dotenv::dotenv;
use std::env;
use transmission_rpc::TransClient;
use transmission_rpc::types::{
    BasicAuth, Result, RpcResponse, SessionGet, Torrent, TorrentGetField, Torrents,
};

#[tokio::main]
async fn main() -> Result<()> {
    //dotenv().ok();
    match dotenv() {
        Ok(path) => println!("Loaded .env from: {}", path.display()),
        Err(e) => eprintln!("Failed to load .env: {}", e),
    }

    let url = env::var("TRANSMISSION_URL")
        .unwrap_or("http://localhost:9091/transmission/rpc".to_string());
    // Replace with your Transmission RPC URL (e.g., http://localhost:9091/transmission/rpc)
    let url = url.parse().unwrap();
    let user = env::var("TRANSMISSION_LOGIN").unwrap();
    let password = env::var("TRANSMISSION_PASSWORD").unwrap();

    // Optional basic authentication if enabled on your Transmission daemon
    let basic_auth = BasicAuth { user, password };

    let mut client = TransClient::with_auth(url, basic_auth);

    // Call session_get() as a test
    let response: Result<RpcResponse<SessionGet>> = client.session_get().await;

    match response {
        Ok(res) => println!("Connected successfully! Response: {:?}", res),
        Err(e) => eprintln!("Failed to connect: {:?}", e),
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

    let info: Vec<String> = response_torrents
        .arguments
        .torrents
        .iter()
        .map(|it| {
            format!(
                "{}. {} {} {} {:?} {} {}",
                &it.id.as_ref().unwrap(),
                &it.hash_string.as_ref().unwrap(),
                &it.name.as_ref().unwrap(),
                &it.download_dir.as_ref().unwrap(),
                &it.error.as_ref().unwrap(),
                &it.error_string.as_ref().unwrap(),
                &it.comment.as_ref().unwrap(),
            )
        })
        .collect();
    println!("{:#?}", info);

    Ok(())
}
