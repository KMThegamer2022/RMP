//! Skins and colours from themes.h and ui.m.

use eframe::egui::Color32;

pub fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// The shipping app's skins, in the View → Theme menu order (ui.m), with their themes.h indices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Skin {
    Vlc,
    RmpDefault,
    Modern,
    Seven,
    Xp,
    Mac,
    Linux,
}

pub const SKINS: [Skin; 7] = [Skin::Vlc, Skin::RmpDefault, Skin::Modern, Skin::Seven, Skin::Xp, Skin::Mac, Skin::Linux];

impl Skin {
    pub fn label(self) -> &'static str {
        match self {
            Skin::Vlc => "VLC",
            Skin::RmpDefault => "RMP Default",
            Skin::Modern => "Windows 10/11",
            Skin::Seven => "Windows 7",
            Skin::Xp => "Windows XP",
            Skin::Mac => "Mac",
            Skin::Linux => "Linux",
        }
    }
    /// Stable id for the preference file and the RMP_SPIKE_THEME override.
    pub fn slug(self) -> &'static str {
        match self {
            Skin::Vlc => "vlc",
            Skin::RmpDefault => "rmp-default",
            Skin::Modern => "win10",
            Skin::Seven => "win7",
            Skin::Xp => "xp",
            Skin::Mac => "mac",
            Skin::Linux => "linux",
        }
    }
    /// themes.h index (the shipping app's PlayerTheme value).
    pub fn index(self) -> i32 {
        match self {
            Skin::Modern => 0,
            Skin::Seven => 1,
            Skin::Xp => 2,
            Skin::Mac => 3,
            Skin::RmpDefault => 4,
            Skin::Vlc => 5,
            Skin::Linux => 6,
        }
    }
    pub fn from_slug(s: &str) -> Option<Skin> {
        SKINS.into_iter().find(|k| k.slug() == s.trim())
    }
    pub fn from_index(i: i32) -> Option<Skin> {
        SKINS.into_iter().find(|k| k.index() == i)
    }
    /// Seven and XP use the Windows Media Player-style layout (hasMediaLayout in mac-library.h).
    pub fn has_media_layout(self) -> bool {
        matches!(self, Skin::Seven | Skin::Xp)
    }
    pub fn theme(self) -> Theme {
        // themes.h rows: panel, edge, face, highlight, text, accent, radius. `dark` is the window appearance.
        let (panel, edge, face, highlight, text, accent, radius, dark) = match self {
            Skin::Modern => (0x101010, 0x555555, 0x101010, 0x303030, 0xffffff, 0x399bdb, 0.0, false),
            Skin::Seven => (0xd2e3f1, 0x8198ae, 0xb5ccdf, 0xffffff, 0x123150, 0x2673b5, 6.0, false),
            Skin::Xp => (0xece9d8, 0x30537f, 0xe4dfcc, 0xfffef7, 0x171710, 0x245edb, 3.0, false),
            Skin::Mac => (0x303034, 0x696970, 0x525257, 0x525257, 0xffffff, 0x286bd1, 10.0, true),
            Skin::RmpDefault => (0x171a21, 0x414756, 0x292e39, 0x353b49, 0xf4f5f7, 0xff7848, 8.0, true),
            Skin::Vlc => (0xf2f2f2, 0xb4b4b4, 0xf8f8f8, 0xffffff, 0x535353, 0x57bb39, 2.0, false),
            Skin::Linux => (0x101611, 0x37483b, 0x142019, 0x213527, 0xd3e8d7, 0x83dc9b, 0.0, true),
        };
        Theme {
            panel: rgb(panel),
            edge: rgb(edge),
            face: rgb(face),
            highlight: rgb(highlight),
            text: rgb(text),
            accent: rgb(accent),
            radius,
            dark,
        }
    }
    /// Window background from applyTheme: XP blue, Windows 10/11 light grey, otherwise the panel colour.
    pub fn window_background(self) -> Color32 {
        match self {
            // applyTheme sets 0x3978db / 0xf5f5f5, but the shipping window's title bar shows the panel
            // colour in every skin (the content view covers the rest), so the title bar uses that.
            _ => self.theme().panel,
        }
    }
    /// Spacing of the transport row (applyTheme: SEVEN 2, MODERN 18, otherwise 8).
    pub fn transport_spacing(self) -> f32 {
        match self {
            Skin::Seven => 2.0,
            Skin::Modern => 18.0,
            _ => 8.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub panel: Color32,
    pub edge: Color32,
    pub face: Color32,
    pub highlight: Color32,
    pub text: Color32,
    pub accent: Color32,
    pub radius: f32,
    pub dark: bool,
}
