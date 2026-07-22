//! Terminal layer.
//!
//! Owns everything that talks to the terminal: capability detection (`caps`), the ANSI/SGR
//! builder + color downsampling (`ansi`, Phase 1), raw-mode input (`input`), and escape
//! sequences (`osc`). We build directly on the terminal rather than a TUI framework so the
//! render path — first-paint latency and damage frames — stays under our control (ADR 0003).

pub mod ansi;
pub mod caps;
