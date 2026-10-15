use serde::{Deserialize, Serialize};

/// How finished files get into the library.
/// Default follows the open decision in the design: hardlink, so seeding continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MoveMode {
    #[default]
    Hardlink,
    Move,
    Copy,
}

/// Folder icon method, per desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IconMethod {
    /// Detect the desktop from `XDG_CURRENT_DESKTOP`, fall back to `.directory`.
    #[default]
    Auto,
    /// Write `Icon=folder.png` into `.directory` (KDE and most file managers).
    Directory,
    /// `gio set -t string <folder> metadata::custom-icon` (GNOME/Nautilus).
    Gio,
    Off,
}

/// Plain poster, or the poster composited onto a folder shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IconStyle {
    #[default]
    Plain,
    FolderShape,
}
