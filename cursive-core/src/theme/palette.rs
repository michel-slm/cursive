use super::Color;
use enum_map::{enum_map, Enum, EnumMap};
#[cfg(feature = "toml")]
use log::warn;

use std::ops::{Index, IndexMut};
use std::str::FromStr;

// Use AHash instead of the slower SipHash
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

/// Error parsing a color.
#[derive(Debug)]
pub struct NoSuchColor;

impl std::fmt::Display for NoSuchColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not parse the given color")
    }
}

impl std::error::Error for NoSuchColor {}

/// Color configuration for the application.
///
/// Assign each color role an actual color.
///
/// It implements `Index` and `IndexMut` to access and modify this mapping:
///
/// It also implements [`Extend`] to update a batch of colors at
/// once.
///
/// # Example
///
/// ```rust
/// # use cursive_core::theme::Palette;
/// use cursive_core::theme::{BaseColor::*, Color::*, PaletteColor::*};
///
/// let mut palette = Palette::default();
///
/// assert_eq!(palette[Background], Dark(Blue));
/// palette[Shadow] = Light(Red);
/// assert_eq!(palette[Shadow], Light(Red));
///
/// let colors = vec![(Shadow, Dark(Green)), (Primary, Light(Blue))];
/// palette.extend(colors);
/// assert_eq!(palette[Shadow], Dark(Green));
/// assert_eq!(palette[Primary], Light(Blue));
/// ```
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Palette {
    basic: EnumMap<PaletteColor, Color>,
    custom: HashMap<String, PaletteNode>,
}

/// A node in the palette tree.
///
/// This describes a value attached to a custom keyword in the palette.
///
/// This can either be a color, or a nested namespace with its own mapping.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum PaletteNode {
    /// A single color.
    Color(Color),
    /// A group of values bundled in the same namespace.
    ///
    /// Namespaces can be merged in the palette with `Palette::merge`.
    Namespace(HashMap<String, PaletteNode>),
}

// Basic usage: only use basic colors
impl Index<PaletteColor> for Palette {
    type Output = Color;

    fn index(&self, palette_color: PaletteColor) -> &Color {
        &self.basic[palette_color]
    }
}

// We can alter existing color if needed (but why?...)
impl IndexMut<PaletteColor> for Palette {
    fn index_mut(&mut self, palette_color: PaletteColor) -> &mut Color {
        &mut self.basic[palette_color]
    }
}

impl Palette {
    /// Returns a custom color from this palette.
    ///
    /// Returns `None` if the given key was not found.
    pub fn custom<'a>(&'a self, key: &str) -> Option<&'a Color> {
        self.custom.get(key).and_then(|node| {
            if let PaletteNode::Color(ref color) = *node {
                Some(color)
            } else {
                None
            }
        })
    }

    /// Returns a new palette where the given namespace has been merged.
    ///
    /// All values in the namespace will override previous values.
    #[must_use]
    pub fn merge(&self, namespace: &str) -> Self {
        let mut result = self.clone();

        if let Some(&PaletteNode::Namespace(ref palette)) =
            self.custom.get(namespace)
        {
            // Merge `result` and `palette`
            for (key, value) in palette.iter() {
                match *value {
                    PaletteNode::Color(color) => result.set_color(key, color),
                    PaletteNode::Namespace(ref map) => {
                        result.add_namespace(key, map.clone())
                    }
                }
            }
        }

        result
    }

    /// Sets the color for the given key.
    ///
    /// This will update either the basic palette or the custom values.
    pub fn set_color(&mut self, key: &str, color: Color) {
        if self.set_basic_color(key, color).is_err() {
            self.custom
                .insert(key.to_string(), PaletteNode::Color(color));
        }
    }

    /// Sets a basic color from its name.
    ///
    /// Returns `Err(())` if `key` is not a known `PaletteColor`.
    pub fn set_basic_color(
        &mut self,
        key: &str,
        color: Color,
    ) -> Result<(), NoSuchColor> {
        PaletteColor::from_str(key).map(|c| self.basic[c] = color)
    }

    /// Adds a color namespace to this palette.
    pub fn add_namespace(
        &mut self,
        key: &str,
        namespace: HashMap<String, PaletteNode>,
    ) {
        self.custom
            .insert(key.to_string(), PaletteNode::Namespace(namespace));
    }

    /// Fills `palette` with the colors from the given `table`.
    #[cfg(feature = "toml")]
    pub(crate) fn load_toml(&mut self, table: &toml::value::Table) {
        // TODO: use serde for that?
        // Problem: toml-rs doesn't do well with Enums...

        for (key, value) in iterate_toml(table) {
            match value {
                PaletteNode::Color(color) => self.set_color(key, color),
                PaletteNode::Namespace(map) => self.add_namespace(key, map),
            }
        }
    }
}

impl Extend<(PaletteColor, Color)> for Palette {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (PaletteColor, Color)>,
    {
        for (k, v) in iter {
            (*self)[k] = v;
        }
    }
}

/// Returns the default palette for a cursive application.
///
/// * `Background` => `Dark(Blue)`
/// * `Shadow` => `Dark(Black)`
/// * `View` => `Dark(White)`
/// * `Primary` => `Dark(Black)`
/// * `Secondary` => `Dark(Blue)`
/// * `Tertiary` => `Light(White)`
/// * `TitlePrimary` => `Dark(Red)`
/// * `TitleSecondary` => `Dark(Yellow)`
/// * `Highlight` => `Dark(Red)`
/// * `HighlightInactive` => `Dark(Blue)`
/// * `HighlightText` => `Dark(White)`
impl Default for Palette {
    fn default() -> Palette {
        use self::PaletteColor::*;
        use crate::theme::BaseColor::*;
        use crate::theme::Color::*;

        Palette {
            basic: enum_map! {
                Background => Dark(Blue),
                Shadow => Dark(Black),
                View => Dark(White),
                Primary => Dark(Black),
                Secondary => Dark(Blue),
                Tertiary => Light(White),
                TitlePrimary => Dark(Red),
                TitleSecondary => Light(Blue),
                Highlight => Dark(Red),
                HighlightInactive => Dark(Blue),
                HighlightText => Dark(White),
            },
            custom: HashMap::default(),
        }
    }
}

// Iterate over a toml
#[cfg(feature = "toml")]
fn iterate_toml(
    table: &toml::value::Table,
) -> impl Iterator<Item = (&str, PaletteNode)> {
    table.iter().flat_map(|(key, value)| {
        let node = match value {
            toml::Value::Table(table) => {
                // This should define a new namespace
                // Treat basic colors as simple string.
                // We'll convert them back in the merge method.
                let map = iterate_toml(table)
                    .map(|(key, value)| (key.to_string(), value))
                    .collect();
                // Should we only return something if it's non-empty?
                Some(PaletteNode::Namespace(map))
            }
            toml::Value::Array(colors) => {
                // This should be a list of colors - just pick the first valid one.
                colors
                    .iter()
                    .flat_map(toml::Value::as_str)
                    .flat_map(Color::parse)
                    .map(PaletteNode::Color)
                    .next()
            }
            toml::Value::String(color) => {
                // This describe a new color - easy!
                Color::parse(color).map(PaletteNode::Color)
            }
            other => {
                // Other - error?
                warn!(
                    "Found unexpected value in theme: {} = {:?}",
                    key, other
                );
                None
            }
        };

        node.map(|node| (key.as_str(), node))
    })
}

/// Color entry in a palette.
///
/// Each `PaletteColor` is used for a specific role in a default application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum PaletteColor {
    /// Color used for the application background.
    Background,
    /// Color used for View shadows.
    Shadow,
    /// Color used for View backgrounds.
    View,
    /// Primary color used for the text.
    Primary,
    /// Secondary color used for the text.
    Secondary,
    /// Tertiary color used for the text.
    Tertiary,
    /// Primary color used for title text.
    TitlePrimary,
    /// Secondary color used for title text.
    TitleSecondary,
    /// Color used for highlighting text.
    Highlight,
    /// Color used for highlighting inactive text.
    HighlightInactive,
    /// Color used for highlighted text
    HighlightText,
}

impl PaletteColor {
    /// Given a palette, resolve `self` to a concrete color.
    pub fn resolve(self, palette: &Palette) -> Color {
        palette[self]
    }

    /// Returns an iterator on all possible palette colors.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }
}

impl FromStr for PaletteColor {
    type Err = NoSuchColor;

    fn from_str(s: &str) -> Result<Self, NoSuchColor> {
        use PaletteColor::*;

        Ok(match s {
            "Background" | "background" => Background,
            "Shadow" | "shadow" => Shadow,
            "View" | "view" => View,
            "Primary" | "primary" => Primary,
            "Secondary" | "secondary" => Secondary,
            "Tertiary" | "tertiary" => Tertiary,
            "TitlePrimary" | "title_primary" => TitlePrimary,
            "TitleSecondary" | "title_secondary" => TitleSecondary,
            "Highlight" | "highlight" => Highlight,
            "HighlightInactive" | "highlight_inactive" => HighlightInactive,
            "HighlightText" | "highlight_text" => HighlightText,
            _ => return Err(NoSuchColor),
        })
    }
}
