egui-winit 0.29.1, MIT OR Apache-2.0, vendored from crates.io.
Linux IME change backported from upstream PR #5198, merge commit 3ffe1ed7742a8927caee818c49018f59c93b0615.
https://github.com/emilk/egui/pull/5198
Additional Linux XIM handling preserves a pending preedit across a Disabled notification, because Bamboo can send that before Commit. A commit without preceding preedit now starts a composition at the current caret, avoiding dropped second words. Native offline IBus Bamboo test verified "tiếng việt". Other parts of the crate are unchanged.
