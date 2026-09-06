#include "buttercomp2.h"
#include <cmath>
#include <algorithm>
#include <cstdlib>

// ButterComp2 implementation based on Airwindows algorithm
// Original: https://github.com/airwindows/airwindows (MIT License)
// Adapted for FFI integration with Rust NIH-plug

// #18: program-dependent release adaptation. A per-block crest factor
// (peak/RMS of the dry input, stereo-linked) is mapped to a release-time
// multiplier and smoothed at block rate, so the release ballistic softens
// on transient-dense material (drum bus) and tightens on sustained
// material (vocal bus) instead of using one fixed shape for everything.
// See docs/adr/0014-buttercomp2-adaptive-envelope.md.
namespace {
// Crest factor (dB) treated as "neutral" — a typical well-mixed bus signal.
constexpr double kCrestRefDb = 9.0;
// dB span mapped to one octave (2x) of release-time scaling.
constexpr double kCrestRangeDb = 12.0;
// Bounds on the release-time multiplier — kept modest ("slight" per the
// issue) rather than the ~27x span FetCompressor's auto-release uses.
constexpr double kMinReleaseScale = 0.6;
constexpr double kMaxReleaseScale = 2.0;
// Block-rate smoothing time constant for the release-scale estimate itself,
// so it tracks the program material's character rather than jittering
// block-to-block.
constexpr double kCrestSmoothTcSeconds = 0.1;
} // namespace

struct ButterComp2State {
    // Sample rate
    double sample_rate;

    // Parameters (0.0 to 1.0 range)
    double compress;
    double output;
    double dry_wet;

    // #18: true restores the original fixed-shape envelope follower exactly
    // (release-time scale pinned to 1.0).
    bool adaptive_envelope_bypass;
    // Smoothed release-time multiplier derived from the input's crest
    // factor. Persists across blocks; reset to 1.0 (neutral) in
    // buttercomp2_reset so a transport restart doesn't carry over a stale
    // learned value from unrelated prior material.
    double crest_scale_smoothed;

    // Per-channel state variables (Left/Right)
    double control_A_pos[2];
    double control_A_neg[2];
    double control_B_pos[2];
    double control_B_neg[2];
    double target_pos[2];
    double target_neg[2];
    double avg_A[2];
    double avg_B[2];

    // Additional state for dynamics
    double dyn_A[2];
    double dyn_B[2];

    // FPFLIP for dithering
    int fpflip;
};

extern "C" {

ButterComp2State* buttercomp2_create(double sample_rate) {
    ButterComp2State* state = (ButterComp2State*)calloc(1, sizeof(ButterComp2State));
    if (!state) return nullptr;
    
    state->sample_rate = sample_rate;
    
    // Initialize parameters
    state->compress = 0.0;
    state->output = 0.5;    // 0.5 = unity gain
    state->dry_wet = 1.0;   // 1.0 = fully wet

    // #18: adaptive envelope on by default (matches the roadmap's "stop
    // sounding digital" default-on precedent set by #16's hysteresis).
    state->adaptive_envelope_bypass = false;
    state->crest_scale_smoothed = 1.0; // neutral — matches the fixed baseline

    // Initialize state variables to zero (calloc handles this)
    state->fpflip = 1;
    
    return state;
}

void buttercomp2_destroy(ButterComp2State* state) {
    if (state) {
        free(state);
    }
}

void buttercomp2_set_compress(ButterComp2State* state, double compress) {
    if (state) {
        state->compress = std::max(0.0, std::min(1.0, compress));
    }
}

void buttercomp2_set_output(ButterComp2State* state, double output) {
    if (state) {
        state->output = std::max(0.0, std::min(1.0, output));
    }
}

void buttercomp2_set_dry_wet(ButterComp2State* state, double dry_wet) {
    if (state) {
        state->dry_wet = std::max(0.0, std::min(1.0, dry_wet));
    }
}

void buttercomp2_set_adaptive_envelope_bypass(ButterComp2State* state, bool bypass) {
    if (state) {
        state->adaptive_envelope_bypass = bypass;
    }
}

void buttercomp2_reset(ButterComp2State* state) {
    if (!state) return;
    
    // Reset all state variables
    for (int ch = 0; ch < 2; ch++) {
        state->control_A_pos[ch] = 0.0;
        state->control_A_neg[ch] = 0.0;
        state->control_B_pos[ch] = 0.0;
        state->control_B_neg[ch] = 0.0;
        state->target_pos[ch] = 0.0;
        state->target_neg[ch] = 0.0;
        state->avg_A[ch] = 0.0;
        state->avg_B[ch] = 0.0;
        state->dyn_A[ch] = 0.0;
        state->dyn_B[ch] = 0.0;
    }
    state->crest_scale_smoothed = 1.0;
}

void buttercomp2_process_stereo(ButterComp2State* state, 
                                float* left_channel, 
                                float* right_channel, 
                                int num_samples) {
    if (!state || !left_channel || !right_channel) return;
    
    // Convert parameters to Airwindows ranges
    double compress_amount = state->compress * 14.0; // 0-14 dB range
    double output_gain = state->output * 2.0;        // 0-2x gain range
    double wet = state->dry_wet;
    double dry = 1.0 - wet;
    
    // Processing constants
    const double one_over_sample_rate = 1.0 / state->sample_rate;
    const double release_speed = 0.001 * one_over_sample_rate;

    // #18: measure this block's crest factor from the dry input (before any
    // compression flattens it) and derive a release-time scale from it.
    // Uses the *previous* call's smoothed value to process this block —
    // fully causal, no added latency — then updates the smoothed value from
    // this block's own stats for the next call.
    double release_scale = state->adaptive_envelope_bypass ? 1.0 : state->crest_scale_smoothed;
    if (num_samples > 0) {
        double peak = 0.0;
        double sum_sq = 0.0;
        for (int i = 0; i < num_samples; i++) {
            double l = (double)left_channel[i];
            double r = (double)right_channel[i];
            peak = std::max(peak, std::max(fabs(l), fabs(r)));
            sum_sq += l * l + r * r;
        }
        double rms = std::sqrt(sum_sq / (2.0 * num_samples));
        double crest_db = 20.0 * std::log10(std::max(peak, 1e-9) / std::max(rms, 1e-9));
        // Inverted: higher crest factor (transient/bursty) -> smaller scale
        // -> slower dynamic_release_speed -> softer release. Lower crest
        // factor (sustained) -> larger scale -> tighter/faster release.
        double target_scale = std::pow(2.0, (kCrestRefDb - crest_db) / kCrestRangeDb);
        // std::min/max, not std::clamp (C++17-only) — matches this file's
        // existing clamping style and avoids depending on the toolchain's
        // default C++ standard on non-MSVC targets.
        target_scale = std::max(kMinReleaseScale, std::min(kMaxReleaseScale, target_scale));

        double block_duration_s = (double)num_samples / state->sample_rate;
        double smooth_coeff = std::exp(-block_duration_s / kCrestSmoothTcSeconds);
        state->crest_scale_smoothed =
            smooth_coeff * state->crest_scale_smoothed + (1.0 - smooth_coeff) * target_scale;
    }
    const double dynamic_release_speed = release_speed * release_scale;

    for (int i = 0; i < num_samples; i++) {
        // Process both channels
        float* channels[2] = {&left_channel[i], &right_channel[i]};
        
        for (int ch = 0; ch < 2; ch++) {
            double input_sample = (double)(*channels[ch]);
            double dry_sample = input_sample;
            
            // Airwindows ButterComp2 algorithm implementation
            
            // Input conditioning
            input_sample *= 1.0 + compress_amount * 0.1;
            
            // Bi-polar compression with butterfly processing
            double pos_target = fabs(input_sample);
            double neg_target = -fabs(input_sample);
            
            // Control smoothing with different time constants
            state->target_pos[ch] = (state->target_pos[ch] * 0.999) + (pos_target * 0.001);
            state->target_neg[ch] = (state->target_neg[ch] * 0.999) + (neg_target * 0.001);
            
            // Four compressors in butterfly configuration
            double control_A = state->target_pos[ch] * compress_amount * 0.1;
            double control_B = state->target_neg[ch] * compress_amount * 0.1;
            
            // Apply compression with different characteristics
            if (input_sample > 0.0) {
                state->control_A_pos[ch] += (control_A - state->control_A_pos[ch]) * dynamic_release_speed;
                input_sample /= (1.0 + state->control_A_pos[ch]);
            } else {
                state->control_A_neg[ch] += (control_B - state->control_A_neg[ch]) * dynamic_release_speed;
                input_sample /= (1.0 + fabs(state->control_A_neg[ch]));
            }

            // Second stage of compression (parallel)
            double abs_sample = fabs(input_sample);
            if (abs_sample > state->avg_A[ch]) {
                state->avg_A[ch] = abs_sample;
            } else {
                state->avg_A[ch] = (state->avg_A[ch] * 0.999) + (abs_sample * 0.001);
            }

            // Apply dynamic compression
            double comp_ratio = 1.0 + (compress_amount * 0.1);
            if (abs_sample > state->avg_A[ch] * 1.1) {
                input_sample /= comp_ratio;
            }
            
            // Output stage.
            // No inline hard clip: the dedicated Punch clipper at the end of
            // the signal chain owns ceiling management. Clipping here robs the
            // downstream clipper of headroom and aliases at native sample rate.
            input_sample *= output_gain;

            // Dry/Wet mix
            double output_sample = (dry_sample * dry) + (input_sample * wet);
            
            // Dithering for final output
            state->fpflip = !state->fpflip;
            if (state->fpflip) {
                output_sample += (double(rand()) / RAND_MAX - 0.5) * 1.0e-10;
            }
            
            *channels[ch] = (float)output_sample;
        }
    }
}

} // extern "C"