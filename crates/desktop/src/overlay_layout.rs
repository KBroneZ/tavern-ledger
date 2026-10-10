//! What the user can change in the overlay (T-302): where each panel is and
//! how big, which panels show, the theme and the opacity. Pure data and
//! rules, no window code, so every rule here has a test.
//!
//! Positions are in logical pixels of the overlay window, which covers the
//! game's monitor. A panel with no saved rectangle stays in the default stack
//! (top right since T-306, clear of the game's leaderboard).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::leaderboard;
use crate::screen_fit::Area;

/// The panels of the overlay. `Legend` explains the source marks, so it can
/// move and resize but never be hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Panel {
    Status,
    Tribes,
    Opponents,
    Legend,
}

impl Panel {
    pub fn hideable(self) -> bool {
        self != Panel::Legend
    }
}

pub const MIN_WIDTH: f64 = 160.0;
pub const MAX_WIDTH: f64 = 800.0;
pub const MIN_HEIGHT: f64 = 40.0;
/// A panel's top edge keeps at least this much on screen, so it can always be
/// grabbed again.
const GRAB_HEIGHT: f64 = 32.0;
/// Panels whose size the user never set take their height from their content.
const FALLBACK_SCREEN: (f64, f64) = (1920.0, 1080.0);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    /// `None`: as tall as the content. When set it is a minimum: content that
    /// does not fit makes the panel taller, so nothing is ever cut off.
    #[serde(default)]
    pub h: Option<f64>,
}

impl Rect {
    fn finite(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.w.is_finite()
            && self.h.is_none_or(f64::is_finite)
    }

    /// Fits the rectangle to a screen, or `None` if its numbers are not numbers.
    pub fn clamped(&self, screen: (f64, f64)) -> Option<Rect> {
        if !self.finite() {
            return None;
        }
        let (sw, sh) = sane_screen(screen);
        let w = self.w.clamp(MIN_WIDTH.min(sw), MAX_WIDTH.min(sw));
        let h = self.h.map(|h| h.clamp(MIN_HEIGHT.min(sh), sh));
        let grab = h.unwrap_or(MIN_HEIGHT).min(GRAB_HEIGHT);
        Some(Rect {
            x: self.x.clamp(0.0, (sw - w).max(0.0)),
            y: self.y.clamp(0.0, (sh - grab).max(0.0)),
            w,
            h,
        })
    }
}

fn sane_screen(screen: (f64, f64)) -> (f64, f64) {
    if screen.0.is_finite() && screen.1.is_finite() && screen.0 >= 1.0 && screen.1 >= 1.0 {
        screen
    } else {
        FALLBACK_SCREEN
    }
}

/// The panels the user has placed. The others stay in the default stack.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub panels: BTreeMap<Panel, Rect>,
}

impl Layout {
    fn clamped(&self, screen: (f64, f64)) -> Layout {
        Layout {
            panels: self
                .panels
                .iter()
                .filter_map(|(p, r)| r.clamped(screen).map(|r| (*p, r)))
                .collect(),
        }
    }

    /// The same arrangement on a screen of another size.
    fn scaled(&self, from: (f64, f64), to: (f64, f64)) -> Layout {
        let (from, to) = (sane_screen(from), sane_screen(to));
        let (kx, ky) = (to.0 / from.0, to.1 / from.1);
        Layout {
            panels: self
                .panels
                .iter()
                .map(|(p, r)| {
                    let r = Rect {
                        x: r.x * kx,
                        y: r.y * ky,
                        w: r.w * kx,
                        h: r.h.map(|h| h * ky),
                    };
                    (*p, r)
                })
                .collect(),
        }
    }
}

pub fn resolution_key(screen: (f64, f64)) -> String {
    let (w, h) = sane_screen(screen);
    format!("{}x{}", w.round() as i64, h.round() as i64)
}

fn parse_resolution(key: &str) -> Option<(f64, f64)> {
    let (w, h) = key.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Smoked glass with amber lamplight, the Lamplight glass look (D-052).
    /// The retired themes (Combat Round, Tavern, Parchment) read as this one.
    #[default]
    #[serde(alias = "round", alias = "tavern", alias = "parchment")]
    Glass,
    /// Black and white with strong colours.
    Contrast,
}

pub const THEMES: [Theme; 2] = [Theme::Glass, Theme::Contrast];

type Rgb = (u8, u8, u8);

/// Every colour the overlay draws text with, and the panel background they
/// sit on. A test checks each text colour against the background.
pub struct Palette {
    pub panel: Rgb,
    pub text: Rgb,
    pub dim: Rgb,
    pub accent: Rgb,
    pub won: Rgb,
    pub lost: Rgb,
    pub amber: Rgb,
    pub info: Rgb,
    pub muted: Rgb,
    pub danger: Rgb,
    /// Borders and dividers: not text.
    pub border: Rgb,
}

impl Palette {
    fn text_colours(&self) -> [(&'static str, Rgb); 8] {
        [
            ("text", self.text),
            ("dim", self.dim),
            ("accent", self.accent),
            ("won", self.won),
            ("lost", self.lost),
            ("amber", self.amber),
            ("info", self.info),
            ("muted", self.muted),
        ]
    }
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::Glass => "Glass (dark)",
            Theme::Contrast => "High contrast",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Theme::Glass => "glass",
            Theme::Contrast => "contrast",
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Theme::Glass => Palette {
                panel: (0x1e, 0x16, 0x12),
                text: (0xf7, 0xef, 0xe6),
                dim: (0xcb, 0xbd, 0xaf),
                accent: (0xff, 0xd0, 0x8a),
                won: (0x8f, 0xe0, 0xa8),
                lost: (0xff, 0x9a, 0x8a),
                amber: (0xff, 0xb2, 0x4a),
                info: (0xa9, 0xc8, 0xff),
                muted: (0xc4, 0xb6, 0xa8),
                danger: (0xff, 0xd0, 0xc6),
                border: (0x9a, 0x6a, 0x30),
            },
            Theme::Contrast => Palette {
                panel: (0x00, 0x00, 0x00),
                text: (0xff, 0xff, 0xff),
                dim: (0xea, 0xea, 0xea),
                accent: (0xff, 0xf0, 0x00),
                won: (0x7a, 0xff, 0x7a),
                lost: (0xff, 0xb0, 0xa8),
                amber: (0xff, 0xc4, 0x00),
                info: (0x8a, 0xdc, 0xff),
                muted: (0xdc, 0xdc, 0xdc),
                danger: (0xff, 0xc0, 0xc0),
                border: (0xff, 0xff, 0xff),
            },
        }
    }

    /// The CSS custom properties the overlay page sets for this theme.
    pub fn css_vars(self, opacity: f64) -> BTreeMap<&'static str, String> {
        let p = self.palette();
        let hex = |(r, g, b): Rgb| format!("#{r:02x}{g:02x}{b:02x}");
        let mut vars = BTreeMap::new();
        vars.insert(
            "--panel-bg",
            format!(
                "rgba({}, {}, {}, {:.2})",
                p.panel.0, p.panel.1, p.panel.2, opacity
            ),
        );
        vars.insert("--panel-solid", hex(p.panel));
        for (name, colour) in p.text_colours() {
            let key = match name {
                "text" => "--text",
                "dim" => "--dim",
                "accent" => "--accent",
                "won" => "--won",
                "lost" => "--lost",
                "amber" => "--amber",
                "info" => "--info",
                _ => "--muted",
            };
            vars.insert(key, hex(colour));
        }
        vars.insert("--danger", hex(p.danger));
        vars.insert("--border", hex(p.border));
        vars
    }
}

#[cfg(test)]
fn linear(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
fn luminance((r, g, b): Rgb) -> f64 {
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

#[cfg(test)]
/// WCAG contrast ratio between two colours, 1 to 21.
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
/// `fg` drawn at `alpha` over `behind`.
fn blend(fg: Rgb, alpha: f64, behind: Rgb) -> Rgb {
    let mix = |f: u8, b: u8| (f64::from(f) * alpha + f64::from(b) * (1.0 - alpha)).round() as u8;
    (
        mix(fg.0, behind.0),
        mix(fg.1, behind.1),
        mix(fg.2, behind.2),
    )
}

/// Opacity of the panels' background: below this the game shows through too
/// much for any theme to stay readable.
pub const MIN_OPACITY: f64 = 0.8;
pub const MAX_OPACITY: f64 = 1.0;
pub const DEFAULT_OPACITY: f64 = 0.9;

fn clamp_opacity(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(MIN_OPACITY, MAX_OPACITY)
    } else {
        DEFAULT_OPACITY
    }
}

/// What `overlay.json` holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub theme: Theme,
    pub opacity: f64,
    pub hidden: BTreeSet<Panel>,
    /// One layout per screen resolution, by `resolution_key`.
    pub layouts: BTreeMap<String, Layout>,
    /// The resolution of the last layout saved, to start from when the
    /// screen is a new one.
    pub last_resolution: Option<String>,
    /// The leaderboard box the user drew over the game (T-306), per game
    /// window size (`resolution_key` of the client area, physical pixels),
    /// relative to the client area. Without one the measured default is used.
    pub leaderboards: BTreeMap<String, Area>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enabled: false,
            theme: Theme::default(),
            opacity: DEFAULT_OPACITY,
            hidden: BTreeSet::new(),
            layouts: BTreeMap::new(),
            last_resolution: None,
            leaderboards: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// Reads the file's text. Anything that cannot be read gives the
    /// defaults and says so; the values that can be read are made safe.
    pub fn parse(text: &str) -> Result<Settings, String> {
        let mut settings: Settings = serde_json::from_str(text).map_err(|_| {
            "The overlay settings file could not be read; the defaults are used.".to_string()
        })?;
        settings.opacity = clamp_opacity(settings.opacity);
        settings.hidden.retain(|p| p.hideable());
        settings
            .layouts
            .retain(|key, _| parse_resolution(key).is_some());
        settings.leaderboards = std::mem::take(&mut settings.leaderboards)
            .into_iter()
            .filter_map(|(key, area)| {
                let client = parse_resolution(&key)?;
                Some((key, leaderboard::fit_area(area, client)?))
            })
            .collect();
        Ok(settings)
    }

    pub fn to_text(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// The layout to use on this screen: the one saved for it, else the last
    /// one saved on another screen scaled to this one, else the default
    /// stack. Always fits the screen.
    pub fn layout_for(&self, screen: (f64, f64)) -> Layout {
        let key = resolution_key(screen);
        if let Some(layout) = self.layouts.get(&key) {
            return layout.clamped(screen);
        }
        let from = self
            .last_resolution
            .as_ref()
            .and_then(|k| Some((self.layouts.get(k)?, parse_resolution(k)?)));
        match from {
            Some((layout, from)) => layout.scaled(from, screen).clamped(screen),
            None => Layout::default(),
        }
    }

    /// Saves the placed panels for this screen, made to fit it. Panels with
    /// bad numbers are dropped (back in the stack).
    pub fn set_layout(&mut self, screen: (f64, f64), panels: &BTreeMap<Panel, Rect>) {
        let key = resolution_key(screen);
        let layout = Layout {
            panels: panels.clone(),
        }
        .clamped(screen);
        self.layouts.insert(key.clone(), layout);
        self.last_resolution = Some(key);
    }

    /// Puts every panel back in the default stack and shows all of them, on
    /// every screen; the theme and opacity stay.
    pub fn reset_layout(&mut self) {
        self.layouts.clear();
        self.last_resolution = None;
        self.hidden.clear();
    }

    /// The leaderboard box for a game window of this size (physical pixels,
    /// relative to its client area), and whether the user drew it.
    pub fn leaderboard_for(&self, client: (f64, f64)) -> (Area, bool) {
        let saved = self
            .leaderboards
            .get(&resolution_key(client))
            .and_then(|a| leaderboard::fit_area(*a, client));
        match saved {
            Some(area) => (area, true),
            None => (leaderboard::default_area(client), false),
        }
    }

    /// Keeps the box the user drew for this window size, made to fit it.
    /// A box with bad numbers is not kept.
    pub fn set_leaderboard(&mut self, client: (f64, f64), area: Area) {
        if let Some(area) = leaderboard::fit_area(area, client) {
            self.leaderboards.insert(resolution_key(client), area);
        }
    }

    /// Back to the measured default for this window size.
    pub fn reset_leaderboard(&mut self, client: (f64, f64)) {
        self.leaderboards.remove(&resolution_key(client));
    }

    pub fn set_opacity(&mut self, value: f64) {
        self.opacity = clamp_opacity(value);
    }

    /// Shows or hides a panel. The legend cannot be hidden: it explains the
    /// marks on every value.
    pub fn set_shown(&mut self, panel: Panel, shown: bool) {
        if shown || !panel.hideable() {
            self.hidden.remove(&panel);
        } else {
            self.hidden.insert(panel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: (f64, f64) = (1920.0, 1080.0);
    const HD: (f64, f64) = (1280.0, 720.0);

    fn rect(x: f64, y: f64, w: f64, h: Option<f64>) -> Rect {
        Rect { x, y, w, h }
    }

    fn one(panel: Panel, r: Rect) -> BTreeMap<Panel, Rect> {
        BTreeMap::from([(panel, r)])
    }

    #[test]
    fn a_rectangle_inside_the_screen_is_left_alone() {
        let r = rect(100.0, 200.0, 300.0, Some(150.0));
        assert_eq!(r.clamped(FHD), Some(r));
    }

    #[test]
    fn a_rectangle_off_the_screen_is_pulled_back_in() {
        let r = rect(5000.0, -40.0, 300.0, None).clamped(FHD).unwrap();
        assert_eq!(r.x, 1920.0 - 300.0);
        assert_eq!(r.y, 0.0);
        let r = rect(100.0, 5000.0, 300.0, None).clamped(FHD).unwrap();
        assert!(r.y <= 1080.0 - GRAB_HEIGHT, "the top edge stays grabbable");
    }

    #[test]
    fn sizes_stay_between_the_smallest_and_the_screen() {
        let r = rect(0.0, 0.0, 5.0, Some(1.0)).clamped(FHD).unwrap();
        assert_eq!((r.w, r.h), (MIN_WIDTH, Some(MIN_HEIGHT)));
        let r = rect(0.0, 0.0, 99999.0, Some(99999.0)).clamped(HD).unwrap();
        assert_eq!((r.w, r.h), (MAX_WIDTH, Some(720.0)));
        let r = rect(0.0, 0.0, 99999.0, None)
            .clamped((500.0, 400.0))
            .unwrap();
        assert_eq!(r.w, 500.0, "never wider than a small screen");
    }

    #[test]
    fn numbers_that_are_not_numbers_drop_the_rectangle() {
        assert_eq!(rect(f64::NAN, 0.0, 300.0, None).clamped(FHD), None);
        assert_eq!(rect(0.0, f64::INFINITY, 300.0, None).clamped(FHD), None);
        assert_eq!(rect(0.0, 0.0, 300.0, Some(f64::NAN)).clamped(FHD), None);
    }

    #[test]
    fn a_nonsense_screen_falls_back_to_a_usual_one() {
        let r = rect(0.0, 0.0, 300.0, None).clamped((0.0, f64::NAN));
        assert!(r.is_some());
        assert_eq!(resolution_key((0.0, 0.0)), "1920x1080");
    }

    #[test]
    fn a_layout_is_saved_per_resolution_and_restored() {
        let mut s = Settings::default();
        let r = rect(400.0, 300.0, 320.0, Some(200.0));
        s.set_layout(FHD, &one(Panel::Opponents, r));
        let back = Settings::parse(&s.to_text().unwrap()).unwrap();
        assert_eq!(back.layout_for(FHD).panels[&Panel::Opponents], r);
    }

    #[test]
    fn saving_makes_the_layout_fit_the_screen() {
        let mut s = Settings::default();
        s.set_layout(FHD, &one(Panel::Tribes, rect(9000.0, 9000.0, 9000.0, None)));
        let r = s.layout_for(FHD).panels[&Panel::Tribes];
        assert!(r.x + r.w <= 1920.0 && r.y < 1080.0 && r.w <= MAX_WIDTH);
    }

    #[test]
    fn a_new_resolution_starts_from_the_last_layout_scaled_and_on_screen() {
        let mut s = Settings::default();
        s.set_layout(
            FHD,
            &one(Panel::Status, rect(1500.0, 900.0, 300.0, Some(100.0))),
        );
        let on_hd = s.layout_for(HD).panels[&Panel::Status];
        assert!(on_hd.x + on_hd.w <= 1280.0, "{on_hd:?}");
        assert!(on_hd.y + 32.0 <= 720.0, "{on_hd:?}");
        assert!(on_hd.x > 800.0, "kept roughly where it was, scaled");
    }

    #[test]
    fn a_smaller_screen_clamps_a_layout_saved_for_a_bigger_one() {
        let mut s = Settings::default();
        s.set_layout(FHD, &one(Panel::Status, rect(1800.0, 1000.0, 300.0, None)));
        // Same key as FHD but the file was edited or the screen is now small.
        let r = s.layouts[&resolution_key(FHD)]
            .clamped((800.0, 600.0))
            .panels;
        assert!(r.is_empty() || r[&Panel::Status].x <= 800.0);
        let small = s.layout_for((800.0, 600.0)).panels[&Panel::Status];
        assert!(small.x + small.w <= 800.0 && small.y <= 600.0);
    }

    #[test]
    fn with_nothing_saved_every_panel_is_in_the_default_stack() {
        assert!(Settings::default().layout_for(FHD).panels.is_empty());
    }

    #[test]
    fn reset_puts_everything_back_and_keeps_the_theme() {
        let mut s = Settings {
            theme: Theme::Contrast,
            ..Settings::default()
        };
        s.set_layout(FHD, &one(Panel::Status, rect(10.0, 10.0, 200.0, None)));
        s.set_shown(Panel::Tribes, false);
        s.reset_layout();
        assert!(s.layout_for(FHD).panels.is_empty());
        assert!(s.hidden.is_empty());
        assert_eq!(s.theme, Theme::Contrast);
    }

    #[test]
    fn panels_can_be_hidden_but_not_the_legend() {
        let mut s = Settings::default();
        s.set_shown(Panel::Opponents, false);
        s.set_shown(Panel::Legend, false);
        assert!(s.hidden.contains(&Panel::Opponents));
        assert!(!s.hidden.contains(&Panel::Legend));
        s.set_shown(Panel::Opponents, true);
        assert!(s.hidden.is_empty());
    }

    #[test]
    fn opacity_stays_in_range() {
        let mut s = Settings::default();
        s.set_opacity(0.0);
        assert_eq!(s.opacity, MIN_OPACITY);
        s.set_opacity(7.0);
        assert_eq!(s.opacity, MAX_OPACITY);
        s.set_opacity(f64::NAN);
        assert_eq!(s.opacity, DEFAULT_OPACITY);
    }

    #[test]
    fn the_file_from_t_301_still_loads() {
        let s = Settings::parse(r#"{"enabled":true}"#).unwrap();
        assert!(s.enabled);
        assert_eq!(s.theme, Theme::Glass);
        assert_eq!(s.opacity, DEFAULT_OPACITY);
    }

    #[test]
    fn retired_themes_read_as_glass_and_are_saved_as_glass() {
        for old in ["round", "tavern", "parchment"] {
            let s = Settings::parse(&format!(r#"{{"theme":"{old}"}}"#)).unwrap();
            assert_eq!(s.theme, Theme::Glass, "{old}");
            assert_eq!(serde_json::to_string(&s.theme).unwrap(), r#""glass""#);
        }
        let s = Settings::parse(r#"{"theme":"contrast"}"#).unwrap();
        assert_eq!(s.theme, Theme::Contrast);
    }

    #[test]
    fn glass_is_the_default_and_high_contrast_stays() {
        assert_eq!(Theme::default(), Theme::Glass);
        assert_eq!(THEMES, [Theme::Glass, Theme::Contrast]);
    }

    #[test]
    fn a_file_that_is_not_settings_says_so() {
        for bad in [
            "",
            "not json",
            "[1,2]",
            r#"{"theme":"neon"}"#,
            r#"{"enabled":"yes"}"#,
        ] {
            let err = Settings::parse(bad).unwrap_err();
            assert!(err.contains("could not be read"), "{bad}");
        }
    }

    #[test]
    fn values_that_load_but_are_unsafe_are_made_safe() {
        let s = Settings::parse(
            r#"{"opacity":9,"hidden":["legend","tribes"],"layouts":{"nonsense":{"panels":{}}}}"#,
        )
        .unwrap();
        assert_eq!(s.opacity, MAX_OPACITY);
        assert_eq!(s.hidden, BTreeSet::from([Panel::Tribes]));
        assert!(s.layouts.is_empty());
    }

    #[test]
    fn a_rectangle_saved_off_screen_is_fitted_when_read() {
        let s = Settings::parse(
            r#"{"layouts":{"1920x1080":{"panels":{"status":{"x":-500,"y":99999,"w":3,"h":null}}}}}"#,
        )
        .unwrap();
        let r = s.layout_for(FHD).panels[&Panel::Status];
        assert!(r.x >= 0.0 && r.y < 1080.0 && r.w >= MIN_WIDTH);
    }

    #[test]
    fn every_theme_has_a_label_an_id_and_a_full_set_of_variables() {
        for theme in THEMES {
            assert!(!theme.label().is_empty() && !theme.id().is_empty());
            let vars = theme.css_vars(0.9);
            for name in [
                "--panel-bg",
                "--panel-solid",
                "--text",
                "--dim",
                "--accent",
                "--won",
                "--lost",
                "--amber",
                "--info",
                "--muted",
                "--danger",
                "--border",
            ] {
                assert!(vars.contains_key(name), "{} lacks {name}", theme.id());
            }
        }
    }

    #[test]
    fn the_theme_ids_match_what_the_file_holds() {
        for theme in THEMES {
            let text = serde_json::to_string(&theme).unwrap();
            assert_eq!(text, format!("\"{}\"", theme.id()));
        }
    }

    /// Text must stay readable whatever the game shows behind a panel, at the
    /// most see-through setting: checked over black, white and mid grey.
    #[test]
    fn every_text_colour_has_enough_contrast_in_every_theme() {
        const BEHIND: [Rgb; 3] = [(0, 0, 0), (255, 255, 255), (128, 128, 128)];
        for theme in THEMES {
            let p = theme.palette();
            for behind in BEHIND {
                let bg = blend(p.panel, MIN_OPACITY, behind);
                for (name, colour) in p.text_colours() {
                    let ratio = contrast(colour, bg);
                    assert!(
                        ratio >= 4.5,
                        "{} {name} {colour:?} over {behind:?}: {ratio:.2}",
                        theme.id()
                    );
                }
            }
            let solid = contrast(p.danger, p.panel);
            assert!(solid >= 4.5, "{} danger text: {solid:.2}", theme.id());
        }
    }

    #[test]
    fn the_leaderboard_box_is_the_default_until_the_user_draws_one() {
        let client = (3840.0, 2160.0);
        let mut s = Settings::default();
        let (area, custom) = s.leaderboard_for(client);
        assert!(!custom);
        assert_eq!(area, leaderboard::default_area(client));
        let drawn = Area {
            x: 400.0,
            y: 300.0,
            w: 200.0,
            h: 1500.0,
        };
        s.set_leaderboard(client, drawn);
        assert_eq!(s.leaderboard_for(client), (drawn, true));
        assert!(!s.leaderboard_for((1920.0, 1080.0)).1, "per window size");
        let back = Settings::parse(&s.to_text().unwrap()).unwrap();
        assert_eq!(back.leaderboard_for(client), (drawn, true));
        s.reset_leaderboard(client);
        assert!(!s.leaderboard_for(client).1);
    }

    #[test]
    fn a_leaderboard_box_off_the_window_or_broken_is_fitted_or_dropped() {
        let s = Settings::parse(
            r#"{"leaderboards":{"1920x1080":{"x":-50,"y":2000,"w":100,"h":600},"nonsense":{"x":0,"y":0,"w":1,"h":1}}}"#,
        )
        .unwrap();
        assert_eq!(s.leaderboards.len(), 1);
        let (area, custom) = s.leaderboard_for((1920.0, 1080.0));
        assert!(
            custom && area.x >= 0.0 && area.bottom() <= 1080.0,
            "{area:?}"
        );
        let mut s = Settings::default();
        s.set_leaderboard(
            (1920.0, 1080.0),
            Area {
                x: f64::NAN,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
        );
        assert!(s.leaderboards.is_empty());
        assert!(Settings::parse(r#"{"leaderboards":{"1920x1080":{"x":"a"}}}"#).is_err());
    }

    #[test]
    fn the_contrast_formula_matches_known_values() {
        assert!((contrast((0, 0, 0), (255, 255, 255)) - 21.0).abs() < 0.01);
        assert!((contrast((255, 255, 255), (255, 255, 255)) - 1.0).abs() < 0.01);
    }

    #[test]
    fn the_high_contrast_theme_clears_the_stricter_bar() {
        let p = Theme::Contrast.palette();
        for (name, colour) in p.text_colours() {
            let worst = [(0, 0, 0), (255, 255, 255)]
                .iter()
                .map(|b| contrast(colour, blend(p.panel, MIN_OPACITY, *b)))
                .fold(f64::MAX, f64::min);
            assert!(worst >= 7.0, "{name}: {worst:.2}");
        }
    }
}
