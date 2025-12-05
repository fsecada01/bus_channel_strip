// src/styles.rs
// CSS styling for reusable components

pub const COMPONENT_STYLES: &str = r#"

/* 500 Series Lunchbox base styles */
:root {
    font-family: "Noto Sans";
    background-color: #0a0a0a;
    color: #ffffff;
}

/* Lunchbox chassis styling */
.lunchbox-chassis {
    background: linear-gradient(145deg, #1a1a1a, #2a2a2a);
    border: 3px solid #444444;
    border-radius: 12px;
    box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.8);
}

.chassis-header {
    background: linear-gradient(145deg, #333333, #444444);
    border-bottom: 2px solid #555555;
    padding: 12px;
    border-radius: 8px 8px 0 0;
}

.chassis-brand {
    font-size: 24px;
    font-weight: 700;
    color: #cccccc;
    letter-spacing: 2px;
}

.chassis-title {
    font-size: 18px;
    font-weight: 500;
    color: #ffffff;
    margin-left: 20px;
}

.master-controls {
    background: rgba(85, 85, 85, 0.3);
    padding: 8px 16px;
    border-radius: 6px;
    border: 1px solid #666666;
}

.master-label {
    font-size: 12px;
    font-weight: 600;
    color: #cccccc;
    text-transform: uppercase;
    letter-spacing: 1px;
}

.lunchbox-slots {
    padding: 16px;
    background: linear-gradient(145deg, #222222, #2a2a2a);
    border-radius: 0 0 8px 8px;
}

.plugin-title {
    font-size: 24px;
    font-weight: 300;
    text-align: center;
    color: #ffffff;
    height: 40px;
    margin-bottom: 16px;
}

/* 500 Series Module Slots */
.module-slot {
    background: linear-gradient(145deg, #d0d0d0, #e8e8e8);
    border: 2px solid #999999;
    border-radius: 8px;
    padding: 16px 12px;
    margin: 2px;
    box-shadow: 
        inset 0 2px 4px rgba(255, 255, 255, 0.3),
        inset 0 -2px 4px rgba(0, 0, 0, 0.2),
        0 4px 8px rgba(0, 0, 0, 0.3);
    transition: all 0.2s ease;
}

.module-slot:hover {
    box-shadow: 
        inset 0 2px 4px rgba(255, 255, 255, 0.4),
        inset 0 -2px 4px rgba(0, 0, 0, 0.3),
        0 6px 12px rgba(0, 0, 0, 0.4);
}

.module-header {
    text-align: center;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid #aaaaaa;
}

.module-name {
    font-size: 14px;
    font-weight: 700;
    color: #222222;
    letter-spacing: 1px;
    text-transform: uppercase;
}

.module-type {
    font-size: 10px;
    font-weight: 500;
    color: #666666;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    margin-top: 2px;
}

.section-label {
    font-size: 9px;
    font-weight: 600;
    color: #444444;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    text-align: center;
    margin-bottom: 4px;
}

/* Module section components - legacy support */
.module-section {
    background-color: #2a2a2a;
    border-radius: 8px;
    padding: 12px;
    margin: 4px;
    border: 1px solid #3a3a3a;
    transition: border-color 0.2s ease;
}

.module-section:hover {
    border-color: #4a4a4a;
}

.module-title {
    font-size: 16px;
    font-weight: 500;
    text-align: center;
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.section-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    margin-bottom: 6px;
}

/* Module themes with enhanced styling */
.api5500-theme {
    border-color: #40a0d0;
    background: linear-gradient(145deg, #2a2a2a, #2f3540);
}

.api5500-theme .module-title {
    color: #40a0d0;
    text-shadow: 0 0 8px rgba(64, 160, 208, 0.3);
}

.buttercomp2-theme {
    border-color: #ff9640;
    background: linear-gradient(145deg, #2a2a2a, #3a2f28);
}

.buttercomp2-theme .module-title {
    color: #ff9640;
    text-shadow: 0 0 8px rgba(255, 150, 64, 0.3);
}

.pultec-theme {
    border-color: #ffd700;
    background: linear-gradient(145deg, #2a2a2a, #3a3628);
}

.pultec-theme .module-title {
    color: #ffd700;
    text-shadow: 0 0 8px rgba(255, 215, 0, 0.3);
}

.dynamic-eq-theme {
    border-color: #66cc66;
    background: linear-gradient(145deg, #2a2a2a, #28362a);
}

.dynamic-eq-theme .module-title {
    color: #66cc66;
    text-shadow: 0 0 8px rgba(102, 204, 102, 0.3);
}

.transformer-theme {
    border-color: #cc6633;
    background: linear-gradient(145deg, #2a2a2a, #362a28);
}

.transformer-theme .module-title {
    color: #cc6633;
    text-shadow: 0 0 8px rgba(204, 102, 51, 0.3);
}

.punch-theme {
    border-color: #00a0ff;
    background: linear-gradient(145deg, #2a2a3a, #3a3050);
}

.punch-theme .module-title {
    color: #00a0ff;
    text-shadow: 0 0 8px rgba(0, 160, 255, 0.3);
}

.master-section {
    background: linear-gradient(145deg, #333333, #3a3a3a);
    border-radius: 8px;
    padding: 16px;
    margin: 8px;
    border: 2px solid #555555;
}

/* Parameter control components */
.param-group {
    margin: 4px 2px;
    padding: 8px;
    background-color: rgba(255, 255, 255, 0.02);
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    min-width: 120px;
    flex: 1;
}

.param-control {
    margin: 3px;
    padding: 4px;
    border-radius: 3px;
    transition: background-color 0.15s ease;
    min-width: 85px;
    width: auto;
}

.param-control:hover {
    background-color: rgba(255, 255, 255, 0.05);
}

/* Parameter labels for 500 series modules */
.param-label {
    font-size: 9px;
    color: #333333;
    text-align: center;
    margin-bottom: 4px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

/* Dark param labels for dark chassis areas */
.param-label.dark {
    color: #cccccc;
}

/* Specialized parameter controls */
.frequency-control .param-label {
    color: #40a0d0;
}

.gain-control .param-label {
    color: #ff9640;
}

.ratio-control .param-label {
    color: #66cc66;
}

/* Enhanced bypass button */
.bypass-button {
    background: linear-gradient(145deg, #4a4a4a, #3a3a3a);
    border: 1px solid #666666;
    border-radius: 4px;
    color: #ffffff;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    text-align: center;
    cursor: pointer;
    min-width: 60px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.15s ease;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.bypass-button:hover {
    background: linear-gradient(145deg, #555555, #444444);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
    transform: translateY(-1px);
}

.bypass-button.on {
    background: linear-gradient(145deg, #2a7a2a, #236b23);
    border-color: #4a9a4a;
    color: #ffffff;
    box-shadow: 0 0 12px rgba(42, 122, 42, 0.4);
}

.bypass-button.bypass {
    background: linear-gradient(145deg, #7a2a2a, #6b2323);
    border-color: #9a4a4a;
    color: #ffffff;
    box-shadow: 0 0 12px rgba(122, 42, 42, 0.4);
}

/* Enhanced slider styling */
slider {
    height: 20px;
    background: linear-gradient(145deg, #404040, #353535);
    border-radius: 10px;
    margin: 2px 0px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3);
    transition: all 0.15s ease;
}

slider:hover {
    background: linear-gradient(145deg, #454545, #3a3a3a);
}

slider .track {
    background: linear-gradient(145deg, #606060, #555555);
    border-radius: 10px;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
}

slider .active {
    background: linear-gradient(145deg, #4080ff, #3070ef);
    border-radius: 10px;
    box-shadow: 0 0 8px rgba(64, 128, 255, 0.3);
}

slider .thumb {
    background: linear-gradient(145deg, #ffffff, #e0e0e0);
    border: 1px solid #cccccc;
    border-radius: 50%;
    width: 16px;
    height: 16px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
    transition: all 0.15s ease;
}

slider .thumb:hover {
    background: linear-gradient(145deg, #ffffff, #f0f0f0);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.4);
    transform: scale(1.1);
}

/* Specialized slider themes */
.frequency-slider {
    border: 1px solid rgba(64, 160, 208, 0.3);
}

.frequency-slider .active {
    background: linear-gradient(145deg, #40a0d0, #3090c0);
}

.gain-slider {
    border: 1px solid rgba(255, 150, 64, 0.3);
}

.gain-slider .active {
    background: linear-gradient(145deg, #ff9640, #ef8630);
}

.ratio-slider {
    border: 1px solid rgba(102, 204, 102, 0.3);
}

.ratio-slider .active {
    background: linear-gradient(145deg, #66cc66, #56bc56);
}

/* Scrolling container */
.main-scroll-container {
    overflow-y: auto;
    overflow-x: hidden;
    height: 100%;
    width: 100%;
    padding-right: 8px;
}

/* Scrollbar styling */
scrollbar {
    width: 12px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
}

scrollbar .track {
    background-color: rgba(255, 255, 255, 0.1);
    border-radius: 6px;
}

scrollbar .thumb {
    background-color: rgba(255, 255, 255, 0.3);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
}

scrollbar .thumb:hover {
    background-color: rgba(255, 255, 255, 0.4);
}

/* Module section adjustments for flexible layout */
.module-section {
    background-color: #2a2a2a;
    border-radius: 6px;
    padding: 12px;
    margin: 6px;
    border: 1px solid #3a3a3a;
    transition: border-color 0.2s ease;
    min-height: 160px;
    height: auto;
}

/* Animation and interaction enhancements */
@keyframes glow-pulse {
    0%, 100% {
        box-shadow: 0 0 8px rgba(64, 160, 208, 0.2);
    }
    50% {
        box-shadow: 0 0 16px rgba(64, 160, 208, 0.4);
    }
}

.module-section.active {
    animation: glow-pulse 2s ease-in-out infinite;
}

/* Responsive adjustments */
@media (max-width: 1200px) {
    .param-control {
        width: 80px;
    }
    
    slider {
        width: 70px;
    }
}

@media (max-width: 800px) {
    .module-section {
        padding: 8px;
        margin: 2px;
    }
    
    .param-control {
        width: 70px;
    }
}

"#;