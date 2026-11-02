//! The settings window's colours and widget metrics: dark asphalt with the
//! one DiRT red-orange accent, in the SDK's palette roles
//! (`rea_launcher_ui::theme`).

use rea_launcher_ui::egui::Color32;
use rea_launcher_ui::theme::{CardMetrics, Metrics, Palette, tint};

pub const BLUE: Color32 = Color32::from_rgb(0x17, 0x19, 0x1c); // asphalt panel fill
pub const PANEL: Color32 = Color32::from_rgb(0x26, 0x2a, 0x2e); // card fill
pub const LINE: Color32 = Color32::from_rgb(0x45, 0x4b, 0x52); // control borders
pub const GOLD: Color32 = Color32::from_rgb(0xe0, 0x4b, 0x2f); // the one accent
pub const INK: Color32 = Color32::from_rgb(0xf2, 0xf0, 0xeb); // text
pub const MUTED: Color32 = Color32::from_rgb(0xb4, 0xb0, 0xa8); // secondary text
pub const ON_GOLD: Color32 = Color32::from_rgb(0x1b, 0x1d, 0x20); // text on the accent
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x8a, 0x80);

pub const PALETTE: Palette = Palette {
    base: BLUE,
    panel: PANEL,
    line: LINE,
    ink: INK,
    muted: MUTED,
    action: GOLD,
    action_hover: Color32::from_rgb(0xff, 0xd3, 0x2a),
    on_action: ON_GOLD,
    select: GOLD,
    on_select: ON_GOLD,
    error: ERROR,
    disabled_ink: MUTED,
    disabled_line: LINE,
    focus: INK,
    selection: tint(GOLD, 128),
};

/// The sizes this window's own widgets drew before the SDK had them:
/// roomier look cards and toggle labels on one line.
pub const METRICS: Metrics = Metrics {
    toggle_wrap: false,
    card: CardMetrics {
        pad: 14.0,
        note: 13.5,
        note_top: 32.0,
        ..Metrics::DEFAULT.card
    },
    ..Metrics::DEFAULT
};
