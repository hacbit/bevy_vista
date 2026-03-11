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

#[derive(Component)]
struct BoolDropdownControl {
    field_path: String,
    target: InspectorBindingTarget,
}

struct BoolDropdownDriver;

impl InspectorDriver for BoolDropdownDriver {
    fn id(&self) -> &'static str {
        INSPECTOR_DRIVER_BOOL_DROPDOWN
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let control = DropdownBuilder::new()
            .width(px(100.0))
            .options(bool_options())
            .disabled(true)
            .build(commands, theme);

        commands.entity(control).insert(BoolDropdownControl {
            field_path: field.field_path.clone(),
            target: InspectorBindingTarget::Style,
        });
        control
    }

    fn retarget_control(
        &self,
        commands: &mut Commands,
        control: Entity,
        target: InspectorBindingTarget,
    ) {
        commands
            .entity(control)
            .entry::<BoolDropdownControl>()
            .and_modify(move |mut binding| {
                binding.target = target.clone();
            });
    }

    fn serialize(
        &self,
        _editor: InspectorFieldEditor,
        field: &dyn PartialReflect,
        _theme: Option<&Theme>,
    ) -> Option<String> {
        Some(read_bool_field(field)?.to_string())
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
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
    controls: Query<&BoolDropdownControl>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }

    for change in changes.read() {
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        let value = change.selected == 1;
        let _ = ctx.apply_to_field(
            &control.target,
            &control.field_path,
            InspectorFieldEditor::new(INSPECTOR_DRIVER_BOOL_DROPDOWN),
            None,
            |field| write_bool_field(field, value),
        );
    }
}

fn sync_bool_dropdown_controls(
    ctx: InspectorDriverSyncContext,
    mut controls: Query<(&BoolDropdownControl, &mut Dropdown)>,
) {
    if !ctx.changed() {
        return;
    }

    let Some(selection) = ctx.selection() else {
        for (_, mut dropdown) in controls.iter_mut() {
            dropdown.options = bool_options();
            dropdown.selected = 0;
            dropdown.expanded = false;
            dropdown.disabled = true;
        }
        return;
    };

    for (control, mut dropdown) in controls.iter_mut() {
        dropdown.options = bool_options();
        let Some(source) = selection.source(&control.target, &control.field_path) else {
            dropdown.disabled = true;
            continue;
        };
        let Some(value) = read_bool_field(source) else {
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
