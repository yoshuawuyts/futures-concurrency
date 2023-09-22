use futures_concurrency::future::{Join, TryJoin};
use std::future::ready;
use tokio::time::{sleep, Duration};

async fn process(iter: impl IntoIterator<Item = &i32>, fail: bool) -> Result<Vec<i32>, Vec<()>> {
    if fail {
        return Err(vec![]);
    } else {
        sleep(Duration::from_secs(5)).await;
    }

    Ok(iter
        .into_iter()
        .map(|i| ready(*i))
        .collect::<Vec<_>>()
        .join()
        .await)
}

#[tokio::test]
async fn test() -> Result<(), Vec<()>> {
    let v = (0..10).collect::<Vec<_>>();

    (
        process(v.iter().take(5), true),
        process(v.iter().take(0), false),
    )
        .try_join()
        .await?;

    Ok(())
}
