//! Parameter definitions, split one file per DSP module (issue #21 / roadmap
//! §4.2, §5). Every leaf keeps its original `#[id = "..."]` string via a bare
//! `#[nested]` (no `id_prefix`, no `group`) so host automation and existing
//! DAW sessions are unaffected — see `nice_plug`'s `NestedParams::Inline`.

mod api5500;
mod buttercomp2;
mod dynamic_eq;
mod global;
mod haas;
mod pultec;
mod punch;
mod routing;
mod sheen;
mod transformer;

pub use api5500::Api5500Params;
pub use buttercomp2::ButterComp2Params;
pub use dynamic_eq::DynamicEqParams;
pub use global::GlobalParams;
pub use haas::HaasParams;
pub use pultec::PultecParams;
pub use punch::PunchParams;
pub use routing::RoutingParams;
pub use sheen::SheenParams;
pub use transformer::TransformerParams;

use nice_plug::prelude::*;
#[cfg(feature = "gui")]
use std::sync::Arc;
#[cfg(feature = "gui")]
use vizia_plug::ViziaState;

#[derive(Params)]
pub struct BusChannelStripParams {
    #[nested]
    pub global: GlobalParams,
    #[nested]
    pub api5500: Api5500Params,
    #[nested]
    pub buttercomp2: ButterComp2Params,
    #[nested]
    pub pultec: PultecParams,
    #[nested]
    pub dynamic_eq: DynamicEqParams,
    #[nested]
    pub transformer: TransformerParams,
    #[nested]
    pub punch: PunchParams,
    #[nested]
    pub haas: HaasParams,
    #[nested]
    pub sheen: SheenParams,
    #[nested]
    pub routing: RoutingParams,

    /// GUI window size / HiDPI zoom state. Persisted so the plugin reopens at
    /// the same zoom level across DAW sessions (issue #20).
    #[cfg(feature = "gui")]
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
}

impl Default for BusChannelStripParams {
    fn default() -> Self {
        Self {
            global: GlobalParams::default(),
            api5500: Api5500Params::default(),
            buttercomp2: ButterComp2Params::default(),
            pultec: PultecParams::default(),
            dynamic_eq: DynamicEqParams::default(),
            transformer: TransformerParams::default(),
            punch: PunchParams::default(),
            haas: HaasParams::default(),
            sheen: SheenParams::default(),
            routing: RoutingParams::default(),

            #[cfg(feature = "gui")]
            editor_state: crate::editor::default_state(),
        }
    }
}
