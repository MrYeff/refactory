use avian3d::prelude::PhysicsDebugPlugin;
use avian3d::prelude::PhysicsGizmos;
use bevy::prelude::*;
use bevy::window::Monitor;
use bevy_flycam::prelude::*;
use bevy_infinite_grid::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::FilterQueryInspectorPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;

const TOGGLE_DBG_DISPLAY: KeyCode = KeyCode::F3;

pub fn plugin(app: &mut App) {
    app.init_resource::<ShowDbg>()
        .init_resource::<DbgCfg>()
        .add_plugins(EguiPlugin::default())
        .add_plugins((
            FilterQueryInspectorPlugin::<(Without<Observer>, Without<Monitor>)>::new().run_if(
                |dbg: Option<Res<ShowDbg>>, cfg: Option<Res<DbgCfg>>| {
                    dbg.map(|d| **d).unwrap_or(false)
                        && cfg.map(|c| c.entity_inspector).unwrap_or(false)
                },
            ),
            ResourceInspectorPlugin::<DbgCfg>::new()
                .run_if(|dbg: Option<Res<ShowDbg>>| dbg.map(|d| **d).unwrap_or(false)),
        ))
        .add_plugins(PhysicsDebugPlugin)
        .add_plugins(NoCameraPlayerPlugin)
        .add_plugins(InfiniteGridPlugin)
        .add_systems(Startup, spawn_infinite_grid)
        .add_systems(Update, toggle_show_dbg)
        .add_systems(
            PostUpdate,
            (
                toggle_fly_camera,
                toggle_physics_debug,
                toggle_infinite_grid,
            )
                .run_if(resource_changed::<DbgCfg>),
        );
}

#[derive(Resource, Default, Deref, DerefMut)]
struct ShowDbg(bool);

#[derive(Reflect, Resource, InspectorOptions, Default)]
#[reflect(Resource, InspectorOptions)]
struct DbgCfg {
    entity_inspector: bool,
    show_colliders: bool,
    show_grid: bool,
    flycam: bool,
}

fn toggle_show_dbg(mut dbg: ResMut<ShowDbg>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(TOGGLE_DBG_DISPLAY) {
        dbg.0 = !dbg.0;
    }
}

fn toggle_fly_camera(
    mut commands: Commands,
    cfg: Res<DbgCfg>,
    camera: Single<Entity, With<Camera>>,
) {
    if cfg.flycam {
        commands.entity(*camera).insert(FlyCam);
    } else {
        commands.entity(*camera).remove::<FlyCam>();
    }
}

fn toggle_physics_debug(mut store: ResMut<GizmoConfigStore>, cfg: Res<DbgCfg>) {
    let (config, _group) = store.config_mut::<PhysicsGizmos>();
    config.enabled = cfg.show_colliders;
}

fn spawn_infinite_grid(mut commands: Commands) {
    commands.spawn(InfiniteGridBundle::default());
}

fn toggle_infinite_grid(cfg: Res<DbgCfg>, mut grid: Query<&mut Visibility, With<InfiniteGrid>>) {
    grid.iter_mut().for_each(|mut vis| {
        *vis = if cfg.show_grid {
            Visibility::Visible
        } else {
            Visibility::Hidden
        }
    });
}
