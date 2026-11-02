//! Read-only account probe; never prints names, IDs, credentials or raw responses.
use zalo_rust::{nicknames, session::Session};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    zalo_rust::storage::init_data_directory()?;
    let mut session = Session::new(false, false, false)?;
    session.load_from_file("session.json")?;
    let aliases = nicknames::fetch(&session).await?;
    let threads = zalo_rust::chat::load_threads_cache();
    let matched = threads
        .iter()
        .filter(|t| !t.is_group && aliases.contains_key(&t.id))
        .count();
    let owner = &session.login.as_ref().unwrap().uin;
    nicknames::save(owner, &aliases)?;
    let restored = nicknames::load(owner);
    anyhow::ensure!(restored == aliases, "Alias cache did not roundtrip");
    println!(
        "aliases={} matched_conversations={} encrypted_cache_roundtrip=true",
        aliases.len(),
        matched
    );
    Ok(())
}
