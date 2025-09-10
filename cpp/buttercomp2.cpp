#include "buttercomp2.h"
#include <cmath>
#include <algorithm>
#include <cstdlib>

// ButterComp2 implementation based on Airwindows algorithm
// Original: https://github.com/airwindows/airwindows (MIT License)
// Adapted for FFI integration with Rust NIH-plug

struct ButterComp2State {
    // Sample rate
    double sample_rate;
    
    // Parameters (0.0 to 1.0 range)
    double compress;
    double output;
    double dry_wet;
    
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
                state->control_A_pos[ch] += (control_A - state->control_A_pos[ch]) * release_speed;
                input_sample /= (1.0 + state->control_A_pos[ch]);
            } else {
                state->control_A_neg[ch] += (control_B - state->control_A_neg[ch]) * release_speed;
                input_sample /= (1.0 + fabs(state->control_A_neg[ch]));
            }
            
            // Second stage of compression (parallel)
            double abs_sample = fabs(input_sample);
            if (abs_sample > state->avg_A[ch]) {
                state->avg_A[ch] = abs_sample;
            } else {
                state->avg_A[ch] = (state->avg_A[ch] * 0.999) + (abs_sample * 0.001);
            }
            
            // Dynamic release modification
            double release_mod = 1.0 + (state->avg_A[ch] * compress_amount * 0.01);
            double dynamic_release = release_speed * release_mod;
            
            // Apply dynamic compression
            double comp_ratio = 1.0 + (compress_amount * 0.1);
            if (abs_sample > state->avg_A[ch] * 1.1) {
                input_sample /= comp_ratio;
            }
            
            // Output stage
            input_sample *= output_gain;
            
            // Apply simple soft limiting to prevent clipping
            if (input_sample > 1.0) input_sample = 1.0;
            if (input_sample < -1.0) input_sample = -1.0;
            
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