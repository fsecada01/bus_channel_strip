//! Extracts a full snapshot of every automatable parameter's plain value,
//! keyed by its stable `#[id]` string. Mirrors nice-plug's own internal
//! `GuiContext::get_state()` value-extraction logic (see
//! `nice-plug-0.1.10/src/wrapper/state.rs::serialize_object`) so a captured
//! snapshot is byte-for-byte what `get_state()` would have produced — this
//! is what lets presets round-trip through `GuiContext::set_state()`
//! without any custom parameter-application code.

use std::collections::BTreeMap;

use nice_plug::params::Params;
use nice_plug::plugin::ParamValue;
use nice_plug::prelude::{Param, ParamPtr};

/// Snapshot every parameter on `params` into a `{id: value}` map.
pub fn extract_param_values<P: Params + ?Sized>(params: &P) -> BTreeMap<String, ParamValue> {
    params
        .param_map()
        .into_iter()
        .map(|(id, ptr, _group)| {
            // SAFETY: `ptr` was just handed back by `params.param_map()` and
            // points into `params`, which outlives this closure — the value
            // is read and converted immediately, no pointer escapes.
            let value = unsafe {
                match ptr {
                    ParamPtr::FloatParam(p) => ParamValue::F32((*p).unmodulated_plain_value()),
                    ParamPtr::IntParam(p) => ParamValue::I32((*p).unmodulated_plain_value()),
                    ParamPtr::BoolParam(p) => ParamValue::Bool((*p).unmodulated_plain_value()),
                    ParamPtr::EnumParam(p) => match (*p).unmodulated_plain_id() {
                        Some(id) => ParamValue::String(id.to_owned()),
                        None => ParamValue::I32((*p).unmodulated_plain_value()),
                    },
                }
            };
            (id, value)
        })
        .collect()
}

/// Structural equality for `ParamValue` (the type itself has no `PartialEq`
/// since it wraps host-facing serialization types). Used to drive the
/// preset-vs-current diff indicator.
pub fn param_values_equal(a: &ParamValue, b: &ParamValue) -> bool {
    match (a, b) {
        (ParamValue::F32(x), ParamValue::F32(y)) => x == y,
        (ParamValue::I32(x), ParamValue::I32(y)) => x == y,
        (ParamValue::Bool(x), ParamValue::Bool(y)) => x == y,
        (ParamValue::String(x), ParamValue::String(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_param_values_covers_every_default_param() {
        let params = crate::params::BusChannelStripParams::default();
        let snapshot = extract_param_values(&params);
        // Every #[id] on the struct must show up exactly once.
        assert_eq!(snapshot.len(), params.param_map().len());
    }

    #[test]
    fn param_values_equal_matches_same_variant_same_value() {
        assert!(param_values_equal(
            &ParamValue::F32(1.5),
            &ParamValue::F32(1.5)
        ));
        assert!(param_values_equal(
            &ParamValue::Bool(true),
            &ParamValue::Bool(true)
        ));
        assert!(param_values_equal(
            &ParamValue::String("Soft".into()),
            &ParamValue::String("Soft".into())
        ));
    }

    #[test]
    fn param_values_equal_rejects_different_values_or_variants() {
        assert!(!param_values_equal(
            &ParamValue::F32(1.5),
            &ParamValue::F32(1.6)
        ));
        assert!(!param_values_equal(
            &ParamValue::F32(1.5),
            &ParamValue::Bool(true)
        ));
    }
}
