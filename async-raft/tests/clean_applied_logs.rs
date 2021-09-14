use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_raft::Config;
use async_raft::RaftStorage;
use fixtures::RaftRouter;
use maplit::btreeset;

#[macro_use]
mod fixtures;

/// Logs should be deleted by raft after applying them, on leader and non-voter.
///
/// - assert logs are deleted on leader aftre applying them.
/// - assert logs are deleted on replication target after installing a snapshot.
///
/// RUST_LOG=async_raft,memstore,clean_applied_logs=trace cargo test -p async-raft --test clean_applied_logs
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn clean_applied_logs() -> Result<()> {
    let (_log_guard, ut_span) = init_ut!();
    let _ent = ut_span.enter();

    // Setup test dependencies.
    let config = Arc::new(
        Config {
            max_applied_log_to_keep: 2,
            ..Default::default()
        }
        .validate()?,
    );
    let router = Arc::new(RaftRouter::new(config.clone()));

    let mut n_logs = router.new_nodes_from_single(btreeset! {0}, btreeset! {1}).await?;

    router.client_request_many(0, "0", (10 - n_logs) as usize).await;
    n_logs = 10;

    router.wait_for_log(&btreeset! {0,1}, n_logs, timeout(), "write upto 10 logs").await?;

    tracing::info!("--- logs before max_applied_log_to_keep should be cleaned");
    {
        for node_id in 0..1 {
            let sto = router.get_storage_handle(&node_id).await?;
            let logs = sto.get_log_entries(..).await?;
            assert_eq!(2, logs.len(), "node {} should have only {} logs", node_id, 2);
        }
    }

    Ok(())
}

fn timeout() -> Option<Duration> {
    Some(Duration::from_millis(5000))
}