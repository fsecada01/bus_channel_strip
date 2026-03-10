# BCS Mix Advisor — Reaper Installation

## Prerequisites

Install these via **Extensions > ReaPack > Browse Packages** in Reaper:

| Package | Repository |
|---------|-----------|
| **ReaImGui** | ReaTeam Extensions |
| **js_ReaScriptAPI** | ReaTeam Extensions |

## Setup

### 1. Start the advisor service

```bash
just advisor-dev
```

The service runs at `http://localhost:7373`. Keep this terminal open while using Reaper.

### 2. Copy scripts to Reaper

Copy the entire `scripts/bcs_advisor/` folder into your Reaper scripts directory:

| Platform | Path |
|---------|------|
| Windows | `%APPDATA%\REAPER\Scripts\bcs_advisor\` |
| macOS | `~/Library/Application Support/REAPER/Scripts/bcs_advisor/` |

### 3. Register the action

1. In Reaper: **Actions > Show Action List**
2. Click **New action > Load ReaScript**
3. Navigate to `bcs_advisor/main.lua`
4. Optionally assign a keyboard shortcut

### 4. (Optional) Add spectral analyzer

For real-time frequency context in suggestions:

1. Copy `jsfx/bcs_spectrum_reader.jsfx` to your Reaper JSFX folder:
   - Windows: `%APPDATA%\REAPER\Effects\`
2. On your bus track, insert **BCS Spectrum Reader** as the last effect (after Bus Channel Strip)
3. Set its **GMem Offset** slider to `0` (matches the default in `panel.lua`)

## Usage

1. Select your bus track in Reaper
2. Run the **BCS Mix Advisor** action
3. Pick a studio profile from the dropdown (optional)
4. Type a creative brief (e.g. *"Heavy 80s NYC R&B, punchy low-mids, slightly dark top"*)
5. Click **Analyze & Suggest**
6. Review the summary and suggested parameters
7. Click **Apply** to load the suggestions (your original state is saved as A)
8. Use **A/B toggle** to compare original vs suggested with a smooth animated transition

## Troubleshooting

**"Advisor offline"** — Run `just advisor-dev` in the project directory first.

**"No BCS found on selected track"** — Select the track with Bus Channel Strip loaded, then re-run the action.

**Parameters not applying** — Ensure parameter IDs in the suggestion match your current plugin version. Re-run the action after updating the plugin.

**Spectral data not included** — Add `bcs_spectrum_reader.jsfx` after BCS on the bus track and confirm it is running (not bypassed).
