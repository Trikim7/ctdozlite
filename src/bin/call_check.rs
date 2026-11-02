fn main() {
    let report = zalo_rust::call_support::CallSupport::inspect();
    for (name, ready) in report.checks() {
        println!("{name}: {}", if ready { "ready" } else { "missing" });
    }
    println!(
        "voice_ready={} video_ready={}",
        report.ready_voice(),
        report.ready_video()
    );
}
