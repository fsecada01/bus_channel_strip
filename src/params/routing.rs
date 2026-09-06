use nice_plug::prelude::*;

use crate::ModuleType;

#[derive(Params)]
pub struct RoutingParams {
    // Module Ordering Parameters
    #[id = "module_order_1"]
    pub module_order_1: EnumParam<ModuleType>,
    #[id = "module_order_2"]
    pub module_order_2: EnumParam<ModuleType>,
    #[id = "module_order_3"]
    pub module_order_3: EnumParam<ModuleType>,
    #[id = "module_order_4"]
    pub module_order_4: EnumParam<ModuleType>,
    #[id = "module_order_5"]
    pub module_order_5: EnumParam<ModuleType>,
    #[id = "module_order_6"]
    pub module_order_6: EnumParam<ModuleType>,
    #[id = "module_order_7"]
    pub module_order_7: EnumParam<ModuleType>,

    // Per-module-type hide flags. Purely GUI state — audio path is unaffected.
    // Non-automatable because these are view preferences, not performance
    // parameters. Saved with the session so hides persist across reopens.
    #[id = "hide_api5500"]
    pub hide_api5500: BoolParam,
    #[id = "hide_buttercomp2"]
    pub hide_buttercomp2: BoolParam,
    #[id = "hide_pultec"]
    pub hide_pultec: BoolParam,
    #[id = "hide_dynamic_eq"]
    pub hide_dynamic_eq: BoolParam,
    #[id = "hide_transformer"]
    pub hide_transformer: BoolParam,
    #[id = "hide_punch"]
    pub hide_punch: BoolParam,
    #[id = "hide_haas"]
    pub hide_haas: BoolParam,
}

impl Default for RoutingParams {
    fn default() -> Self {
        Self {
            // Default order places Haas before Punch so the clipper catches
            // any residual peaks introduced by the widener. Slot 7 is Empty
            // by default — users can drop any module (including DynamicEQ)
            // into it via the rack picker. Existing sessions saved before
            // this default change retain their stored slot 7 value.
            module_order_1: EnumParam::new("Module Order 1", ModuleType::Api5500EQ),
            module_order_2: EnumParam::new("Module Order 2", ModuleType::ButterComp2),
            module_order_3: EnumParam::new("Module Order 3", ModuleType::PultecEQ),
            module_order_4: EnumParam::new("Module Order 4", ModuleType::Transformer),
            module_order_5: EnumParam::new("Module Order 5", ModuleType::Haas),
            module_order_6: EnumParam::new("Module Order 6", ModuleType::Punch),
            module_order_7: EnumParam::new("Module Order 7", ModuleType::Empty),

            // Hide flags — all modules visible by default. Marked non-automatable
            // so hosts don't clutter automation lists with per-module view state.
            hide_api5500: BoolParam::new("Hide API5500", false).non_automatable(),
            hide_buttercomp2: BoolParam::new("Hide ButterComp2", false).non_automatable(),
            hide_pultec: BoolParam::new("Hide Pultec", false).non_automatable(),
            hide_dynamic_eq: BoolParam::new("Hide Dynamic EQ", false).non_automatable(),
            hide_transformer: BoolParam::new("Hide Transformer", false).non_automatable(),
            hide_punch: BoolParam::new("Hide Punch", false).non_automatable(),
            hide_haas: BoolParam::new("Hide Haas", false).non_automatable(),
        }
    }
}
