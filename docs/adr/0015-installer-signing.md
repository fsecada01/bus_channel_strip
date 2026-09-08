# ADR-0015: Installer Code Signing & Notarization Scaffolding

**Status**: Scaffolded, not active (blocked on credentials)

**Deciders**: Project (Claude Code orchestration session), issue #26

---

## Context

Issue #26's DoD requires macOS builds to be code-signed and notarized, and
signed installers (Windows MSI + macOS pkg, #26's installer bullet, PR #41)
to install on a fresh Windows 11 / macOS 14 machine with no security
warnings. Doing this for real requires:

- An Apple Developer ID (Application + Installer certificates, ~$99/yr
  Apple Developer Program membership)
- Apple notarization credentials (an app-specific password or App Store
  Connect API key, tied to that same account)
- A Windows Authenticode code-signing certificate (a paid purchase from a
  CA, or an EV cert from a cloud HSM provider)

None of these exist for this project yet, and none can be fabricated or
substituted — an AI session has no path to obtaining them. Rather than
leave the DoD item entirely untouched, or silently skip it, the CI
workflow has been scaffolded to *use* these credentials the moment they
exist, gated so that their absence today is a clean no-op rather than a
broken or misleading pipeline.

## Decision

`build` job in `.github/workflows/main.yml` gains four job-level `env`
booleans, each computed from whether the corresponding secrets are set:

| Env var | True when these repo secrets are all set |
|---|---|
| `WIN_SIGNING_AVAILABLE` | `WIN_CODESIGN_PFX`, `WIN_CODESIGN_PFX_PASSWORD` |
| `APPLE_SIGNING_AVAILABLE` | `APPLE_CERTIFICATE_P12`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` |
| `APPLE_INSTALLER_SIGNING_AVAILABLE` | `APPLE_INSTALLER_IDENTITY` |
| `APPLE_NOTARIZATION_AVAILABLE` | `APPLE_ID`, `APPLE_ID_PASSWORD`, `APPLE_TEAM_ID` |

Every signing step is gated `if: ... && env.<X>_AVAILABLE == 'true'`. With
no secrets configured, every one of these steps is skipped — the workflow
behaves exactly as it did before this ADR.

**What's wired up in this PR** (works today once the two Windows secrets or
three+ Apple secrets are added, no code changes needed):
- Import the Windows PFX to a temp file, `signtool sign` the built
  `Bus-Channel-Strip.vst3` / `Bus-Channel-Strip.clap` binaries, clean up the
  temp file afterward (`if: always()`).
- Import the Apple `.p12` into a temporary, job-scoped keychain (standard
  CI pattern: `security create-keychain` / `import` /
  `set-key-partition-list`), `codesign --options runtime` the macOS
  `.vst3`/`.clap` bundles, delete the temp keychain afterward
  (`if: always()`).

**Deliberately deferred to a follow-up PR, once #41 (installer packaging)
merges**: signing the built `.msi` itself, `productsign`-ing the `.pkg`,
and `xcrun notarytool submit --wait` + `stapler staple` on the `.pkg`. This
branch is cut from `main`, which doesn't yet have #41's MSI/pkg build
steps — wiring installer-level signing here would mean adding steps that
reference build artifacts that don't exist on this branch. Once #41 is
merged, add those steps (env booleans already exist for gating them:
`APPLE_INSTALLER_SIGNING_AVAILABLE`, `APPLE_NOTARIZATION_AVAILABLE`).

## How to activate (for whoever holds the credentials)

1. Buy/obtain an Apple Developer ID Application + Installer certificate
   pair (Apple Developer Program), export both as one `.p12`.
2. Buy/obtain a Windows Authenticode code-signing certificate as a `.pfx`.
3. In the repo's GitHub Settings → Secrets and variables → Actions, add:
   `WIN_CODESIGN_PFX` (base64 of the `.pfx`), `WIN_CODESIGN_PFX_PASSWORD`,
   `APPLE_CERTIFICATE_P12` (base64 of the `.p12`),
   `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (e.g.
   `"Developer ID Application: Name (TEAMID)"`), `APPLE_INSTALLER_IDENTITY`
   (e.g. `"Developer ID Installer: Name (TEAMID)"`), `APPLE_ID`,
   `APPLE_ID_PASSWORD` (an app-specific password, not the Apple ID's login
   password), `APPLE_TEAM_ID`.
4. Next CI run on those legs will sign automatically — no code changes.

## Consequences

**Easier:** the credential-holder's activation path is "add secrets," not
"write CI code" — the risky, hard-to-test part (keychain import, signtool
invocation, notarytool polling) is already written and will start running
the moment secrets exist.

**Harder / deferred:** the DoD's "no security warnings on a fresh install"
criterion is not yet met — today's builds remain unsigned. The `.msi`/`.pkg`
top-level signing (the part that actually matters most for SmartScreen /
Gatekeeper warnings on the installer itself) needs a follow-up PR once #41
merges.

**Risk:** none of this has been exercised against real credentials — the
`if:` gates mean it has never actually run in CI. The first real run with
secrets configured should be treated as the actual verification, not this
PR's merge.
