//! Local sync utility: prints progress/counts only, never keys, IDs or message contents.
use zalo_rust::{session::Session,phone_sync::{self,Update}};
#[tokio::main]
async fn main()->anyhow::Result<()> {
    zalo_rust::storage::init_data_directory()?;
    let mut session=Session::new(false,false,false)?;session.load_from_file("session.json")?;
    match std::env::args().nth(1).as_deref() {
        Some("--shape")=>{println!("{}",phone_sync::diagnose_shape(&session).await?);return Ok(());},
        Some("--diagnose")=>{let (threads,users)=phone_sync::diagnose(&session).await?;println!("Mapped conversations: {threads}; users: {users}");return Ok(());},
        Some("--self-test")=>{phone_sync::bridge_self_test(&session).await?;println!("Bridge self-test passed");return Ok(());},
        Some("--run")=>{},_=>anyhow::bail!("Use --diagnose, --self-test or --run (close the GUI before --run)"),
    }
    let (mut events,commands)=zalo_rust::socket::spawn_with_commands(session.clone());
    let (bus,_)=tokio::sync::broadcast::channel(1024);let receiver=bus.subscribe();
    tokio::spawn(async move{while let Some(event)=events.recv().await {
        for control in phone_sync::controls(&event.payload) {
            let d=&control["data"];eprintln!("sync-control act={} data_object={} has_sync_id={} status={:?} batch_index={:?}",control["act"].as_str().unwrap_or("unknown"),d.is_object(),d.get("syncId").is_some(),d.get("status").and_then(|v|v.as_i64()),d.get("idx").and_then(|v|v.as_u64()));
        }
        if matches!(event.cmd,590..=592){eprintln!("sync-ack command={} code={:?}",event.cmd,event.payload.get("error_code").and_then(|v|v.as_i64()));}
        let _=bus.send(event);
    }});
    let (_cancel,rx)=tokio::sync::watch::channel(false);
    let result=phone_sync::run(session,commands,receiver,rx,|update|match update {
        Update::Status(status)=>println!("{status}"),Update::Threads(threads)=>println!("Saved {} messages",threads.iter().map(|t|t.messages.len()).sum::<usize>()),
    }).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("{}",result?);Ok(())
}
