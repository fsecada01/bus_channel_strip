# Airwindows ButterComp2 Source Code Analysis

## Repository Information
- **Repository**: https://github.com/airwindows/airwindows
- **License**: MIT License (Copyright 2016 airwindows)
- **Plugin Type**: VST Audio Effect (2 inputs, 2 outputs)
- **Unique ID**: 'btcq'

## Source Files Located

### Windows VST Version
- **Main File**: `/plugins/WinVST/ButterComp2/ButterComp2.cpp`
- **Header File**: `/plugins/WinVST/ButterComp2/ButterComp2.h`

### Linux VST Version
- **Main File**: `/plugins/LinuxVST/src/ButterComp2/ButterComp2.cpp`
- **Header File**: `/plugins/LinuxVST/src/ButterComp2/ButterComp2.h`
- **Processing File**: `/plugins/LinuxVST/src/ButterComp2/ButterComp2Proc.cpp` (contains processReplacing implementation)

## Algorithm Overview

ButterComp2 implements a unique **bi-polar, interleaved compression** system with the following characteristics:

### Core Design
- **Four distinct compressors per channel** working in parallel
- **Two compressors** sensitive to positive swing, alternating every sample
- **Two compressors** sensitive to negative swing, alternating every sample
- **Butterfly processing pattern** created by interleaved switching at Nyquist frequency

### Key Technical Features
1. **Bi-polar Processing**: Handles positive and negative waveform halves differently
2. **Interleaved Compressors**: Switches between compressor pairs at sample rate
3. **Dynamic Release Control**: Modifies release time based on output signal level
4. **Class AB Operation**: Push-pull processing between +1 and -1 range

## Parameters

The plugin has 3 main parameters:

### Parameter A: Compress
- Controls compression amount/intensity
- Range: 0-14 dB typical

### Parameter B: Output
- Output gain control (improvement over original ButterComp)
- Scaled by factor of 2.0

### Parameter C: Dry/Wet
- Mix between processed and original signal
- Allows for parallel compression effects

## Class Definition (ButterComp2.h)

```cpp
enum {
    kParamA = 0,      // Compress
    kParamB = 1,      // Output
    kParamC = 2,      // Dry/Wet
    kNumParameters = 3
};

const int kNumPrograms = 0;
const int kNumInputs = 2;
const int kNumOutputs = 2;
const unsigned long kUniqueId = 'btcq';

class ButterComp2 : public AudioEffectX {
    // Member variables for compression state
    // Left channel
    double controlAposL, controlAnegL;
    double controlBposL, controlBnegL; 
    double targetposL, targetnegL;
    double lastOutputL;
    
    // Right channel
    double controlAposR, controlAnegR;
    double controlBposR, controlBnegR;
    double targetposR, targetnegR;
    double lastOutputR;
    
    // Control variables
    bool flip;
    float A, B, C;
    
    // Dithering
    uint32_t fpdL, fpdR;
};
```

## Algorithm Implementation Details

### Processing Flow
1. **Input Processing**: Extract left/right channels, apply small random noise for "live air"
2. **Compression Calculation**: 
   - Process positive and negative components separately
   - Use dynamic divisor that adapts to output levels
   - Alternate between control sets A and B using flip variable
3. **Non-linear Scaling**: Apply power functions for musical compression curve
4. **Output Processing**: Apply wet/dry mix and output gain with 32-bit dithering

### Sonic Characteristics
- **Gentle Compression**: Very subtle, musical response
- **Tonal Reshaping**: Adds second harmonic where compression is asymmetric
- **Glue Effect**: Evens out waveform bulk, balancing positive/negative
- **Treble Detail**: Characteristic treble reduction on ambiences/reverbs
- **Spatial Enhancement**: Creates "holographic" effect on stereo material

## Dependencies

### Required Headers
```cpp
#include "audioeffectx.h"  // VST SDK
#include <set>
#include <string>
#include <math.h>
```

### VST Framework Dependencies
- AudioEffectX base class
- VST parameter handling system
- Chunk save/load functionality

## FFI Wrapper Considerations for Rust Integration

### Key Points for Implementation
1. **State Management**: Maintain separate state for L/R channels
2. **Sample Rate Dependency**: Algorithm relies on sample-rate switching
3. **Floating Point Precision**: Uses both float and double precision
4. **Dithering**: Implements custom dithering for noise shaping
5. **Parameter Smoothing**: Requires parameter interpolation for smooth changes

### Recommended Approach
1. Extract core compression algorithm from processReplacing
2. Create C-compatible wrapper functions
3. Use Rust FFI to interface with NIH-plug framework
4. Implement parameter automation and state management in Rust
5. Handle sample rate and buffer size changes appropriately

## Next Steps for Integration
1. Download complete source files from GitHub repository
2. Analyze processReplacing function implementation in detail
3. Identify core compression calculation routines
4. Create minimal C wrapper for algorithm
5. Build FFI bindings for Rust NIH-plug integration