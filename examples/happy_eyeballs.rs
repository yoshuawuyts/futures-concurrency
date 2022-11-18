use async_std::io::prelude::*;
use futures::future::TryFutureExt;
use futures_concurrency::prelude::*;
use futures_time::prelude::*;

use async_std::net::TcpStream;
use futures::channel::oneshot;
use futures_time::time::Duration;
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let mut socket = open_tcp_socket("rust-lang.org", 80, 3).await?;
    socket.write_all(b"GET / \r\n").await?;
    let mut res = String::new();
    socket.read_to_string(&mut res).await?;
    println!("{res}");
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
    let (mut sender, mut receiver) = oneshot::channel();
    let mut futures = vec![];

    for attempt in 0..attempts {
        let tcp = TcpStream::connect((addr, port));
        let start_event = receiver.timeout(Duration::from_secs(attempt));
        futures.push(tcp.delay(start_event).map_err(|err| {
            let _ = sender.send(());
            err
        }));
        (sender, receiver) = oneshot::channel();
    }

    let socket = futures
        .race_ok()
        .timeout(Duration::from_secs(attempts + 1))
        .await??;

    Ok(socket)
}
