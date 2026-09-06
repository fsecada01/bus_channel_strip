//! Preset system (issue #21): the `.bcs` schema, parameter-snapshot/diff
//! helpers, the factory preset library, and user save/load against
//! `~/Documents/Bus Channel Strip/Presets/`.

mod factory;
mod io;
mod schema;
mod value_ext;

// Only consumed from `editor.rs`; the submodules' own tests reach their
// internals directly (fully-qualified or `use super::*;`) without going
// through this re-export surface, so under non-gui builds nothing uses it.
#[cfg(feature = "gui")]
pub use factory::factory_presets;
#[cfg(feature = "gui")]
pub use io::{list_user_presets, save_user_preset};
#[cfg(feature = "gui")]
pub use schema::Preset;
#[cfg(feature = "gui")]
pub use value_ext::{extract_param_values, param_values_equal};
