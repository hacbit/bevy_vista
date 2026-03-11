use bevy::prelude::*;
use bevy::reflect::PartialReflect;
use bevy_vista::inspector::{read_bool_field, write_bool_field};
use bevy_vista::prelude::*;

const INSPECTOR_DRIVER_BOOL_DROPDOWN: &str = "example_bool_dropdown";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiPlugin)
        .add_plugins(BoolDropdownInspectorPlugin)
        .add_systems(Startup, setup)
        .run();
}

struct BoolDropdownInspectorPlugin;

impl Plugin for BoolDropdownInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.register_inspector_driver(BoolDropdownDriver);

        app.world_mut()
            .resource_mut::<InspectorEditorRegistry>()
            .register_type_driver::<bool>(INSPECTOR_DRIVER_BOOL_DROPDOWN);
    }
}

struct BoolDropdownDriver;

impl InspectorDriver for BoolDropdownDriver {
    fn id(&self) -> &'static str {
        INSPECTOR_DRIVER_BOOL_DROPDOWN
    }

    fn build(
        &self,
        commands: &mut Commands,
        _field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        DropdownBuilder::new()
            .width(px(100.0))
            .options(bool_options())
            .disabled(true)
            .build(commands, theme)
    }

    fn serialize(&self, field: &dyn PartialReflect) -> Option<String> {
        Some(read_bool_field(field)?.to_string())
    }

    fn apply_serialized(&self, field: &mut dyn PartialReflect, raw: &str) -> bool {
        raw.parse::<bool>()
            .ok()
            .is_some_and(|value| write_bool_field(field, value))
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_bool_dropdown_changes);
        builder.on_sync(sync_bool_dropdown_controls);
    }
}

fn apply_bool_dropdown_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<DropdownChange>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }

    for change in changes.read() {
        if !ctx.is_control(change.entity, INSPECTOR_DRIVER_BOOL_DROPDOWN) {
            continue;
        }
        let value = change.selected == 1;
        let _ = ctx.write_for(change.entity, |field| write_bool_field(field, value));
    }
}

fn sync_bool_dropdown_controls(
    ctx: InspectorDriverSyncContext,
    mut controls: Query<(Entity, &mut Dropdown)>,
) {
    if !ctx.changed() {
        return;
    }

    for (entity, mut dropdown) in controls.iter_mut() {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_BOOL_DROPDOWN) {
            continue;
        }
        dropdown.options = bool_options();
        let Some(value) = ctx.read_for(entity, read_bool_field) else {
            dropdown.selected = 0;
            dropdown.expanded = false;
            dropdown.disabled = true;
            continue;
        };
        dropdown.selected = usize::from(value);
        dropdown.expanded = false;
        dropdown.disabled = false;
    }
}

fn setup(
    mut commands: Commands,
    mut active: ResMut<VistaEditorActive>,
    mut expanded: ResMut<VistaEditorExpanded>,
    mut mode: ResMut<VistaEditorMode>,
    mut document: ResMut<WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
) {
    commands.spawn((Camera2d, IsDefaultUiCamera));

    **active = true;
    **expanded = true;
    *mode = VistaEditorMode::Fullscreen;

    let _ = apply_blueprint_command(
        BlueprintCommand::AddRoot {
            widget_path: "input/checkbox".to_owned(),
        },
        &mut document,
        &widget_registry,
    );
}

fn bool_options() -> Vec<String> {
    vec!["False".to_owned(), "True".to_owned()]
}
