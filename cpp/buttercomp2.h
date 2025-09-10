#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// ButterComp2 FFI wrapper for Rust integration
// Based on Airwindows ButterComp2 algorithm (MIT License)

typedef struct ButterComp2State ButterComp2State;

// Create/destroy ButterComp2 instance
ButterComp2State* buttercomp2_create(double sample_rate);
void buttercomp2_destroy(ButterComp2State* state);

// Set parameters (0.0 to 1.0 range for Rust compatibility)
void buttercomp2_set_compress(ButterComp2State* state, double compress);
void buttercomp2_set_output(ButterComp2State* state, double output);
void buttercomp2_set_dry_wet(ButterComp2State* state, double dry_wet);

// Process stereo audio (in-place)
void buttercomp2_process_stereo(ButterComp2State* state, 
                                float* left_channel, 
                                float* right_channel, 
                                int num_samples);

// Reset state (for parameter changes or initialization)
void buttercomp2_reset(ButterComp2State* state);

#ifdef __cplusplus
}
#endif