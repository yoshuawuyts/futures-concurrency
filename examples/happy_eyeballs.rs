use futures::channel::oneshot;
use futures_concurrency::prelude::*;
use futures_time::prelude::*;

use async_std::net::TcpStream;
use futures_time::time::Duration;
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let _socket = open_tcp_socket("debian.org", 80, 3).await?;
    println!("Successfully opened socket!");
    Ok(())
}

/// Happy eyeballs algorithm!
///
/// 1. Create an iterator for the number of attempts
/// 2. Create a `TcpStream` for each attempt
/// 3. Delay the start of each tcp stream by `N` seconds
/// 4. Collect into a vector of futures
/// 5. Concurrently execute all futures using "race_ok" semantics -
///    on failure, try the next future. Return on success or if all futures
///    fail.
/// 6. Set a timeout for our entire futures construct, so we stop all at the same time.
async fn open_tcp_socket(
    addr: &str,
    port: u16,
    attempts: u64,
) -> Result<TcpStream, Box<dyn Error + Send + Sync + 'static>> {
    let (init_send, init_recv) = oneshot::channel();
    // Make the initial one start up right away
    let _ = init_send.send(());
    let mut futures = vec![];
    let _last_receiver = (0..attempts).fold(init_recv, |event, attempt| {
        let (send, recv) = oneshot::channel();
        futures.push(async move {
            // Wait for the previous one to complete
            let _ = event.timeout(Duration::from_secs(attempt)).await?;

            TcpStream::connect((addr, port)).await.map_err(|e| {
                // If we have an error, tell the next one they can go ahead and start
                let _ = send.send(());
                e
            })
        });
        recv
    });

    let socket = futures
        .race_ok()
        .timeout(Duration::from_secs(attempts + 1))
        .await??;

    Ok(socket)
}
