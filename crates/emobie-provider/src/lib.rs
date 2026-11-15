//! emobie-provider: emobie's emoji search in uQ's launcher. A small session-bus service on
//! `org.emobie.Provider` that serves `org.uq.Launcher1.Provider`, started by the bus when uQ's
//! launcher opens and gone again after a minute without a call.

pub mod activate;
pub mod data;
pub mod inputd;
pub mod prefs;
pub mod provider;
pub mod search;

pub use provider::{Provider, BUS_NAME, INTERFACE, PATH};
