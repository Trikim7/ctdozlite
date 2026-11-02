//! Controlled self-only attachment diagnostic. Never prints credentials or IDs.
use zalo_rust::session::Session;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mode = std::env::args().nth(1).unwrap_or_default();
    anyhow::ensure!(
        mode == "--send-self",
        "Requires explicit --send-self authorization"
    );
    zalo_rust::storage::init_data_directory()?;
    let mut s = Session::new(false, false, false)?;
    s.load_from_file("session.json")?;
    let target = s
        .login
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?
        .send2me_id
        .clone();
    anyhow::ensure!(!target.is_empty(), "Cloud target not available");
    let mut socket = zalo_rust::socket::spawn(s.clone());
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let event = socket
                .recv()
                .await
                .ok_or_else(|| anyhow::anyhow!("Socket closed"))?;
            if event.cmd == 1 {
                return Ok::<(), anyhow::Error>(());
            }
        }
    })
    .await??;
    println!("socket_ready=true");
    let drain = tokio::spawn(async move { while socket.recv().await.is_some() {} });
    let mut cursor = zalo_rust::call::CtrlPollState::default();
    let _ = zalo_rust::call::poll_voip_controls(&s, &mut cursor).await;
    let poll_session = s.clone();
    let poll = tokio::spawn(async move {
        loop {
            let _ = zalo_rust::call::poll_voip_controls(&poll_session, &mut cursor).await;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    });
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("ctdozlite-kiem-tra-gui-tep.txt");
    std::fs::write(
        &path,
        "CTDOZLite: tệp nhỏ kiểm tra chức năng gửi vào Cloud của tôi.\n",
    )?;
    match zalo_rust::chat::send_attachment(&s, &target, false, &path, "").await {
        Ok((response, msg)) => println!(
            "send_self=true code={:?} message_id_present={} remote_url_present={}",
            response.get("error_code"),
            !msg.id.starts_with("out_"),
            msg.link_url
                .as_ref()
                .is_some_and(|u| u.starts_with("https://"))
        ),
        Err(error) => println!("send_self=false error={error}"),
    }
    drain.abort();
    poll.abort();
    Ok(())
}
