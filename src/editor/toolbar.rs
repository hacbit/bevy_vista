use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use rfd::FileDialog;

use crate::asset::VistaUiAsset;
use crate::widget::{ButtonBuilder, LabelBuilder, LabelWidget, WidgetRegistry};

use super::*;

const DEFAULT_DOCUMENT_NAME: &str = "untitled.vista.ron";
const UI_ASSET_DIRECTORY: &str = {
    #[cfg(target_os = "windows")]
    {
        "assets\\ui"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "assets/ui"
    }
};

#[derive(Resource, Default)]
pub(super) struct EditorDocumentPath {
    pub path: Option<PathBuf>,
}

#[derive(Resource, Default)]
pub(super) struct EditorDocumentToolbarState {
    pending_action: Option<DocumentToolbarAction>,
    status: String,
    is_error: bool,
}

#[derive(Clone, Copy)]
enum DocumentToolbarAction {
    Save,
    SaveAs,
    Load,
}

#[derive(Component)]
pub(super) struct MainToolbarSaveButton;

#[derive(Component)]
pub(super) struct MainToolbarLoadButton;

#[derive(Component)]
pub(super) struct MainToolbarSaveAsButton;

#[derive(Component)]
pub(super) struct EditorStatusBar;

#[derive(Component)]
pub(super) struct EditorStatusPathLabel;

#[derive(Component)]
pub(super) struct EditorStatusMessageLabel;

pub(super) fn init_main_toolbar(
    mut commands: Commands,
    toolbar: Single<Entity, With<MainToolbar>>,
    _editor_theme: Res<EditorTheme>,
) {
    let save_button = commands
        .spawn((
            ButtonBuilder::new()
                .text("Save")
                .width(px(72.0))
                .height(px(28.0))
                .build(),
            Name::new("Document Save Button"),
            MainToolbarSaveButton,
        ))
        .observe(on_save_button_click)
        .id();

    let save_as_button = commands
        .spawn((
            ButtonBuilder::new()
                .text("Save As")
                .width(px(84.0))
                .height(px(28.0))
                .build(),
            Name::new("Document Save As Button"),
            MainToolbarSaveAsButton,
        ))
        .observe(on_save_as_button_click)
        .id();

    let load_button = commands
        .spawn((
            ButtonBuilder::new()
                .text("Load")
                .width(px(72.0))
                .height(px(28.0))
                .build(),
            Name::new("Document Load Button"),
            MainToolbarLoadButton,
        ))
        .observe(on_load_button_click)
        .id();

    let controls = commands
        .spawn((
            Name::new("Main Toolbar Controls"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8.0),
                ..default()
            },
        ))
        .add_children(&[save_button, save_as_button, load_button])
        .id();

    commands.entity(*toolbar).add_child(controls);
}

fn on_save_button_click(
    mut event: On<Pointer<Click>>,
    mut toolbar: ResMut<EditorDocumentToolbarState>,
) {
    toolbar.pending_action = Some(DocumentToolbarAction::Save);
    event.propagate(false);
}

fn on_load_button_click(
    mut event: On<Pointer<Click>>,
    mut toolbar: ResMut<EditorDocumentToolbarState>,
) {
    toolbar.pending_action = Some(DocumentToolbarAction::Load);
    event.propagate(false);
}

fn on_save_as_button_click(
    mut event: On<Pointer<Click>>,
    mut toolbar: ResMut<EditorDocumentToolbarState>,
) {
    toolbar.pending_action = Some(DocumentToolbarAction::SaveAs);
    event.propagate(false);
}

pub(super) fn init_status_bar(
    mut commands: Commands,
    content_root: Single<Entity, With<ContentRoot>>,
    editor_theme: Res<EditorTheme>,
    document_path: Res<EditorDocumentPath>,
) {
    let theme = &editor_theme.0;
    let path_label = commands
        .spawn((
            Name::new("Editor Status Path"),
            LabelBuilder::new()
                .text(format!(
                    "File: {}",
                    current_document_path_label(&document_path)
                ))
                .font_size(theme.typography.body_small.font.font_size)
                .color(theme.palette.on_surface_muted)
                .build(),
            EditorStatusPathLabel,
        ))
        .id();

    let message_label = commands
        .spawn((
            Name::new("Editor Status Message"),
            LabelBuilder::new()
                .text("")
                .font_size(theme.typography.body_small.font.font_size)
                .color(theme.palette.on_surface_muted)
                .build(),
            EditorStatusMessageLabel,
        ))
        .id();

    let status_bar = commands
        .spawn((
            Name::new("Editor Status Bar"),
            Node {
                flex_grow: 0.0,
                flex_shrink: 0.0,
                min_height: px(24.0),
                padding: UiRect::axes(px(8.0), px(2.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                border: UiRect::top(px(1.0)),
                ..default()
            },
            BackgroundColor(theme.palette.surface_variant.darker(0.04)),
            BorderColor::all(theme.palette.outline_variant),
            EditorStatusBar,
        ))
        .add_children(&[path_label, message_label])
        .id();

    commands.entity(*content_root).add_child(status_bar);
}

pub(super) fn sync_editor_toolbar_status(
    document_path: Res<EditorDocumentPath>,
    toolbar_state: Res<EditorDocumentToolbarState>,
    mut labels: ParamSet<(
        Single<&mut LabelWidget, With<EditorStatusPathLabel>>,
        Single<&mut LabelWidget, With<EditorStatusMessageLabel>>,
    )>,
    editor_theme: Res<EditorTheme>,
) {
    if !document_path.is_changed() && !toolbar_state.is_changed() && !editor_theme.is_changed() {
        return;
    }

    {
        let mut path_label = labels.p0();
        path_label.text = format!("File: {}", current_document_path_label(&document_path));
        path_label.color = editor_theme.0.palette.on_surface_muted;
    }

    {
        let mut status_label = labels.p1();
        status_label.text = toolbar_state.status.clone();
        status_label.color = if toolbar_state.is_error {
            Color::srgb(0.9, 0.4, 0.4)
        } else {
            editor_theme.0.palette.on_surface_muted
        };
    }
}

pub(super) fn apply_document_toolbar_actions(
    mut toolbar_state: ResMut<EditorDocumentToolbarState>,
    mut document_path: ResMut<EditorDocumentPath>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    mut hierarchy: ResMut<hierarchy::HierarchyState>,
    mut selection: ResMut<VistaEditorSelection>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<crate::inspector::InspectorEditorRegistry>,
) {
    let Some(action) = toolbar_state.pending_action.take() else {
        return;
    };

    match action {
        DocumentToolbarAction::Save => {
            let Some(target_path) = document_path
                .path
                .clone()
                .or_else(|| pick_save_document_path(None))
            else {
                toolbar_state.status = "Save cancelled".to_owned();
                toolbar_state.is_error = false;
                return;
            };

            match save_document(
                &target_path,
                &document,
                &widget_registry,
                &inspector_registry,
            ) {
                Ok(()) => {
                    document_path.path = Some(target_path.clone());
                    toolbar_state.status = format!("Saved {}", target_path.display());
                    toolbar_state.is_error = false;
                }
                Err(error) => {
                    toolbar_state.status = format!("Save failed: {error}");
                    toolbar_state.is_error = true;
                }
            }
        }
        DocumentToolbarAction::SaveAs => {
            let Some(target_path) = pick_save_document_path(document_path.path.as_deref()) else {
                toolbar_state.status = "Save As cancelled".to_owned();
                toolbar_state.is_error = false;
                return;
            };

            match save_document(
                &target_path,
                &document,
                &widget_registry,
                &inspector_registry,
            ) {
                Ok(()) => {
                    document_path.path = Some(target_path.clone());
                    toolbar_state.status = format!("Saved {}", target_path.display());
                    toolbar_state.is_error = false;
                }
                Err(error) => {
                    toolbar_state.status = format!("Save failed: {error}");
                    toolbar_state.is_error = true;
                }
            }
        }
        DocumentToolbarAction::Load => {
            let Some(source_path) = pick_load_document_path(document_path.path.as_deref()) else {
                toolbar_state.status = "Load cancelled".to_owned();
                toolbar_state.is_error = false;
                return;
            };

            match load_document(&source_path) {
                Ok(loaded) => {
                    for node in loaded.nodes.values() {
                        if widget_registry
                            .get_widget_by_path(&node.widget_path)
                            .is_none()
                        {
                            toolbar_state.status =
                                format!("Load failed: unknown widget {}", node.widget_path);
                            toolbar_state.is_error = true;
                            return;
                        }
                    }
                    *document = loaded;
                    document.pending_select = document.roots.first().copied();
                    hierarchy.dirty = true;
                    selection.selected_entity = None;
                    document_path.path = Some(source_path.clone());
                    toolbar_state.status = format!("Loaded {}", source_path.display());
                    toolbar_state.is_error = false;
                }
                Err(error) => {
                    toolbar_state.status = format!("Load failed: {error}");
                    toolbar_state.is_error = true;
                }
            }
        }
    }
}

fn current_document_path_label(document_path: &EditorDocumentPath) -> String {
    document_path
        .path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unsaved>".to_owned())
}

fn pick_load_document_path(current_path: Option<&Path>) -> Option<PathBuf> {
    configure_document_dialog(current_path)
        .pick_file()
        .map(normalize_document_path)
}

fn pick_save_document_path(current_path: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = configure_document_dialog(current_path);
    if current_path.is_none() {
        dialog = dialog.set_file_name(DEFAULT_DOCUMENT_NAME);
    }
    dialog
        .save_file()
        .map(ensure_vista_ron_extension)
        .map(normalize_document_path)
}

fn configure_document_dialog(current_path: Option<&Path>) -> FileDialog {
    let mut dialog = FileDialog::new()
        .add_filter("Vista UI (*.vista.ron)", &["ron"])
        .set_directory(default_document_directory());
    if let Some(path) = current_path {
        let resolved_path = resolve_document_path(path);
        if let Some(parent) = resolved_path.parent() {
            dialog = dialog.set_directory(parent);
        }
        if let Some(file_name) = resolved_path.file_name().and_then(|name| name.to_str()) {
            dialog = dialog.set_file_name(file_name);
        }
    }
    dialog
}

fn ensure_vista_ron_extension(path: PathBuf) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if file_name.ends_with(".vista.ron") {
        path
    } else {
        let mut next = path.into_os_string();
        next.push(".vista.ron");
        PathBuf::from(next)
    }
}

fn save_document(
    path: &Path,
    document: &blueprint::WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
    inspector_registry: &crate::inspector::InspectorEditorRegistry,
) -> Result<(), String> {
    let resolved_path = resolve_document_path(path);
    if let Some(parent) = resolved_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let asset = VistaUiAsset::from(&*document);
    let ron = asset
        .to_ron_string_pretty_compact(widget_registry, inspector_registry)
        .map_err(asset_error_to_string)?;
    fs::write(resolved_path, ron).map_err(|error| error.to_string())
}

fn load_document(path: &Path) -> Result<blueprint::WidgetBlueprintDocument, String> {
    let input =
        fs::read_to_string(resolve_document_path(path)).map_err(|error| error.to_string())?;
    let asset = VistaUiAsset::from_ron_str(&input).map_err(asset_error_to_string)?;
    asset.to_blueprint_document().map_err(asset_error_to_string)
}

fn default_document_directory() -> PathBuf {
    manifest_root().join(ui_asset_root())
}

fn ui_asset_root() -> PathBuf {
    PathBuf::from(UI_ASSET_DIRECTORY)
}

fn manifest_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn resolve_document_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        default_document_directory().join(path)
    }
}

fn normalize_document_path(path: PathBuf) -> PathBuf {
    strip_ui_asset_root(&path).unwrap_or(path)
}

fn strip_ui_asset_root(path: &Path) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(ui_asset_root()) {
        return Some(stripped.to_path_buf());
    }

    let absolute_root = default_document_directory();
    if let Ok(stripped) = path.strip_prefix(&absolute_root) {
        return Some(stripped.to_path_buf());
    }

    None
}

fn asset_error_to_string(error: crate::asset::VistaUiAssetError) -> String {
    match error {
        crate::asset::VistaUiAssetError::UnsupportedVersion(version) => {
            format!("unsupported asset version {version}")
        }
        crate::asset::VistaUiAssetError::DuplicateNodeId(id) => {
            format!("duplicate node id {id}")
        }
        crate::asset::VistaUiAssetError::MissingNode(id) => {
            format!("missing node {id}")
        }
        crate::asset::VistaUiAssetError::MissingChild { parent, child } => {
            format!("missing child {child} referenced by {parent}")
        }
        crate::asset::VistaUiAssetError::InvalidParentLink {
            child,
            expected_parent,
            actual_parent,
        } => format!(
            "invalid parent link for node {child}: expected {:?}, actual {:?}",
            expected_parent, actual_parent
        ),
        crate::asset::VistaUiAssetError::CycleDetected(id) => {
            format!("cycle detected at node {id}")
        }
        crate::asset::VistaUiAssetError::RonDecode(error)
        | crate::asset::VistaUiAssetError::RonEncode(error) => error,
    }
}
