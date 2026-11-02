# X11 IBus Bamboo preedit continuity

Upstream winit 0.30.13, unchanged dependencies/API. The only source patch is in
src/platform_impl/linux/x11/event_processor.rs: accept an ImeEvent::Update after
a word commit even when no new ImeEvent::Start was delivered, and mark composing
active again. Bamboo can keep its XIM preedit session open across word commits;
XLookupString resets the old is_composing guard after the first word, causing
later live updates to be discarded until Space/Enter.

Native offline regression reproduced first word visible but words 2/3 absent
before Space. With this patch, "tiếng", "tiếng việt", and "tiếng việt đang" are
visible before each Space. No raw keystrokes are synthesized into message text.
Upstream LICENSE is retained. See application docs for validation scope.
