//! Theme system: palette + scale tokens with a fast seed-based generator.

use bevy::ecs::hierarchy::ChildOf;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Theme mode (light or dark).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect, Serialize, Deserialize)]
pub enum ThemeMode {
    /// Light theme.
    Light,
    /// Dark theme (default for game applications).
    #[default]
    Dark,
}

/// High-level theme container.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct Theme {
    pub meta: ThemeMeta,
    pub palette: Palette,
    pub spacing: SpacingScale,
    pub radius: RadiusScale,
    pub elevation: ElevationScale,
    pub typography: TypographyScale,
}

#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct EditorTheme(pub Theme);

impl Default for EditorTheme {
    fn default() -> Self {
        Self(Theme::generate(
            ThemeSeed::new("Vista Editor", Color::srgb(0.32, 0.55, 0.78), ThemeMode::Dark)
                .with_secondary(Color::srgb(0.22, 0.68, 0.76))
                .with_neutral(Color::srgb(0.22, 0.24, 0.28))
                .with_description("A compact slate editor theme for Vista.")
                .with_base_font_size(13.0)
                .with_type_scale(1.10),
        ))
    }
}

#[derive(Resource, Clone, Debug)]
pub struct ViewportThemeState {
    pub active_theme_id: Option<String>,
    pub themes: Vec<Theme>,
}

impl Default for ViewportThemeState {
    fn default() -> Self {
        Self {
            active_theme_id: None,
            themes: vec![
                Theme::quick("Rose", Color::srgb(0.78, 0.23, 0.42), ThemeMode::Dark),
                Theme::quick("Ocean", Color::srgb(0.20, 0.56, 0.88), ThemeMode::Dark),
                Theme::quick("Forest", Color::srgb(0.20, 0.66, 0.44), ThemeMode::Dark),
                Theme::quick("Sand", Color::srgb(0.88, 0.62, 0.22), ThemeMode::Light),
            ],
        }
    }
}

impl ViewportThemeState {
    pub fn active_theme(&self) -> Option<&Theme> {
        let active_id = self.active_theme_id.as_deref()?;
        self.themes.iter().find(|theme| theme.meta.id == active_id)
    }

    pub fn options(&self) -> Vec<String> {
        let mut options = Vec::with_capacity(self.themes.len() + 1);
        options.push("None".to_owned());
        options.extend(self.themes.iter().map(|theme| theme.meta.name.clone()));
        options
    }

    pub fn selected_index(&self) -> usize {
        let Some(active_id) = self.active_theme_id.as_deref() else {
            return 0;
        };
        self.themes
            .iter()
            .position(|theme| theme.meta.id == active_id)
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.active_theme_id = if index == 0 {
            None
        } else {
            self.themes
                .get(index - 1)
                .map(|theme| theme.meta.id.clone())
        };
    }
}

#[derive(Component, Clone, Debug, Deref, DerefMut)]
pub struct ThemeScope(pub Theme);

#[derive(Component, Default)]
pub struct ThemeBoundary;

#[derive(Debug, Clone, Reflect)]
pub struct ThemeMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub mode: ThemeMode,
}

/// Quick theme generation input.
#[derive(Debug, Clone)]
pub struct ThemeSeed {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub mode: ThemeMode,

    pub primary: Color,
    pub secondary: Option<Color>,
    pub neutral: Option<Color>,

    pub font: Handle<Font>,
    pub base_font_size: f32,
    pub type_scale: f32,
}

impl ThemeSeed {
    pub fn new(name: impl Into<String>, primary: Color, mode: ThemeMode) -> Self {
        let name = name.into();
        Self {
            id: name.to_lowercase().replace(' ', "-"),
            description: String::new(),
            version: "1.0.0".to_string(),
            name,
            mode,
            primary,
            secondary: None,
            neutral: None,
            font: Handle::default(),
            base_font_size: 16.0,
            type_scale: 1.18,
        }
    }

    pub fn with_secondary(mut self, color: Color) -> Self {
        self.secondary = Some(color);
        self
    }

    pub fn with_neutral(mut self, color: Color) -> Self {
        self.neutral = Some(color);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn with_font(mut self, font: Handle<Font>) -> Self {
        self.font = font;
        self
    }

    pub fn with_base_font_size(mut self, size: f32) -> Self {
        self.base_font_size = size;
        self
    }

    pub fn with_type_scale(mut self, scale: f32) -> Self {
        self.type_scale = scale;
        self
    }
}

impl Theme {
    /// Generate a full theme from a seed.
    pub fn generate(seed: ThemeSeed) -> Self {
        let palette = Palette::from_seed(&seed);
        let spacing = SpacingScale::default();
        let radius = RadiusScale::default();
        let elevation = ElevationScale::from_palette(&palette);
        let typography =
            TypographyScale::from_base(seed.font, seed.base_font_size, seed.type_scale, &palette);

        Self {
            meta: ThemeMeta {
                id: seed.id,
                name: seed.name,
                description: seed.description,
                version: seed.version,
                mode: seed.mode,
            },
            palette,
            spacing,
            radius,
            elevation,
            typography,
        }
    }

    /// Fast default theme from a primary seed color.
    pub fn quick(name: impl Into<String>, primary: Color, mode: ThemeMode) -> Self {
        Theme::generate(ThemeSeed::new(name, primary, mode))
    }

    /// Fast default theme from a hex color (`#RRGGBB` / `RRGGBBAA`).
    pub fn quick_from_hex(
        name: impl Into<String>,
        hex: &str,
        mode: ThemeMode,
    ) -> Result<Self, HexColorError> {
        let color = Color::from(Srgba::hex(hex)?);
        Ok(Theme::quick(name, color, mode))
    }
}

/// Stored themes and optional assets.
pub struct ThemeManager {
    pub current_theme: String,
    pub available_themes: HashMap<String, Theme>,
    pub theme_assets: HashMap<String, Handle<ThemeAsset>>,
}

pub fn resolve_theme<'a>(
    entity: Entity,
    parents: &'a Query<&ChildOf>,
    scopes: &'a Query<&ThemeScope>,
    boundaries: &'a Query<(), With<ThemeBoundary>>,
) -> Option<&'a Theme> {
    let mut current = entity;
    loop {
        if let Ok(theme) = scopes.get(current) {
            return Some(&theme.0);
        }
        if boundaries.contains(current) {
            return None;
        }
        let Ok(parent) = parents.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}

pub fn resolve_theme_or_global<'a>(
    entity: Entity,
    parents: &'a Query<&ChildOf>,
    scopes: &'a Query<&ThemeScope>,
    boundaries: &'a Query<(), With<ThemeBoundary>>,
    global: Option<&'a Theme>,
) -> Option<&'a Theme> {
    let mut current = entity;
    loop {
        if let Ok(theme) = scopes.get(current) {
            return Some(&theme.0);
        }
        if boundaries.contains(current) {
            return None;
        }
        let Ok(parent) = parents.get(current) else {
            return global;
        };
        current = parent.parent();
    }
}

#[derive(Asset, TypePath, Debug, Clone)]
pub struct ThemeAsset {
    pub theme: Theme,
}

#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct Palette {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,

    pub secondary: Color,
    pub on_secondary: Color,

    pub background: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub on_surface_muted: Color,

    pub outline: Color,
    pub outline_variant: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    pub shadow: Color,
    pub scrim: Color,

    pub disabled: Color,
    pub disabled_container: Color,
}

impl Palette {
    pub fn from_seed(seed: &ThemeSeed) -> Self {
        let is_dark = seed.mode == ThemeMode::Dark;

        let primary = seed.primary;
        let secondary = seed
            .secondary
            .unwrap_or_else(|| primary.with_hue(primary.hue() + 28.0).with_saturation(0.6));

        let neutral = seed.neutral.unwrap_or_else(|| {
            let gray = Color::srgb(0.5, 0.5, 0.5);
            primary.mix(&gray, 0.7)
        });

        let background = if is_dark {
            Color::BLACK.mix(&neutral, 0.15)
        } else {
            Color::WHITE.mix(&neutral, 0.06)
        };
        let surface = if is_dark {
            background.lighter(0.05)
        } else {
            background.darker(0.02)
        };
        let surface_variant = if is_dark {
            background.lighter(0.12)
        } else {
            background.darker(0.06)
        };

        let primary_container = if is_dark {
            primary.darker(0.25)
        } else {
            primary.lighter(0.35)
        };

        let on_primary = on_color(primary);
        let on_primary_container = on_color(primary_container);
        let on_secondary = on_color(secondary);
        let on_surface = on_color(surface);
        let on_surface_muted = on_surface.mix(&surface, 0.4);

        let outline = if is_dark {
            Color::WHITE.mix(&neutral, 0.55)
        } else {
            Color::BLACK.mix(&neutral, 0.55)
        };
        let outline_variant = outline.mix(&surface, 0.35);

        let success = tone(Color::srgb(0.15, 0.75, 0.4), is_dark);
        let warning = tone(Color::srgb(0.98, 0.72, 0.2), is_dark);
        let error = tone(Color::srgb(0.92, 0.28, 0.32), is_dark);
        let info = tone(Color::srgb(0.2, 0.6, 0.95), is_dark);

        let shadow = Color::BLACK.with_alpha(if is_dark { 0.75 } else { 0.35 });
        let scrim = Color::BLACK.with_alpha(0.45);

        let disabled = on_surface.with_alpha(0.45);
        let disabled_container = surface_variant.with_alpha(0.6);

        Self {
            primary,
            on_primary,
            primary_container,
            on_primary_container,
            secondary,
            on_secondary,
            background,
            surface,
            surface_variant,
            on_surface,
            on_surface_muted,
            outline,
            outline_variant,
            success,
            warning,
            error,
            info,
            shadow,
            scrim,
            disabled,
            disabled_container,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct SpacingScale {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for SpacingScale {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 12.0,
            lg: 16.0,
            xl: 24.0,
            xxl: 32.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct RadiusScale {
    pub none: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub full: f32,
}

impl Default for RadiusScale {
    fn default() -> Self {
        Self {
            none: 0.0,
            sm: 4.0,
            md: 8.0,
            lg: 12.0,
            full: 999.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct ElevationScale {
    pub sm: ShadowStyle,
    pub md: ShadowStyle,
    pub lg: ShadowStyle,
}

impl ElevationScale {
    pub fn from_palette(palette: &Palette) -> Self {
        let shadow = palette.shadow;
        Self {
            sm: shadow_style(shadow, 0.0, 2.0, 6.0),
            md: shadow_style(shadow, 0.0, 6.0, 16.0),
            lg: shadow_style(shadow, 0.0, 12.0, 28.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct TextStyle {
    pub font: TextFont,
    pub color: Color,
}

#[derive(Debug, Clone, Reflect)]
pub struct TypographyScale {
    pub display_large: TextStyle,
    pub display_medium: TextStyle,
    pub display_small: TextStyle,

    pub headline_large: TextStyle,
    pub headline_medium: TextStyle,
    pub headline_small: TextStyle,

    pub title_large: TextStyle,
    pub title_medium: TextStyle,
    pub title_small: TextStyle,

    pub body_large: TextStyle,
    pub body_medium: TextStyle,
    pub body_small: TextStyle,

    pub label_large: TextStyle,
    pub label_medium: TextStyle,
    pub label_small: TextStyle,
}

impl TypographyScale {
    pub fn from_base(font: Handle<Font>, base: f32, scale: f32, palette: &Palette) -> Self {
        let on_surface = palette.on_surface;
        let mk = |size: f32| TextStyle {
            font: TextFont::from_font_size(size).with_font(font.clone()),
            color: on_surface,
        };

        let s1 = base;
        let s2 = s1 * scale;
        let s3 = s2 * scale;
        let s4 = s3 * scale;
        let s5 = s4 * scale;

        Self {
            display_large: mk(s5 * 1.15),
            display_medium: mk(s5 * 1.02),
            display_small: mk(s5 * 0.9),

            headline_large: mk(s4 * 1.05),
            headline_medium: mk(s4 * 0.95),
            headline_small: mk(s4 * 0.85),

            title_large: mk(s3 * 0.95),
            title_medium: mk(s3 * 0.85),
            title_small: mk(s3 * 0.75),

            body_large: mk(s2 * 0.95),
            body_medium: mk(s2 * 0.85),
            body_small: mk(s2 * 0.75),

            label_large: mk(s1 * 0.95),
            label_medium: mk(s1 * 0.85),
            label_small: mk(s1 * 0.75),
        }
    }
}

fn on_color(color: Color) -> Color {
    if color.luminance() > 0.45 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

fn tone(color: Color, is_dark: bool) -> Color {
    if is_dark {
        color.lighter(0.12)
    } else {
        color.darker(0.04)
    }
}

fn shadow_style(color: Color, x: f32, y: f32, blur: f32) -> ShadowStyle {
    ShadowStyle {
        color,
        x_offset: Val::Px(x),
        y_offset: Val::Px(y),
        spread_radius: Val::Px(0.0),
        blur_radius: Val::Px(blur),
    }
}
