use bevy::{prelude::*, render::render_resource::TextureFormat};
use on_asset_loaded::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(OnAssetLoadedPlugin)
        .add_systems(Startup, (spawn_camera, setup))
        .run()
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// 1. load an image from assets
/// 2. create a grayscale version using the HandleTransformer system param
/// 3. use handles
fn setup(asset_server: Res<AssetServer>, images: Res<Assets<Image>>, mut commands: Commands) {
    let handle_color = asset_server.load("image.png");

    let handle_gray = images.reserve_handle();
    commands.on_loaded_with(
        handle_color.clone(),
        handle_gray.clone(),
        |input: OnLoaded<Image, Handle<Image>>, mut images: ResMut<Assets<Image>>| {
            let img = image_to_grayscale(input.asset);
            images.insert(input.params.id(), img).unwrap();
        },
    );

    // use handles while assets arent yet loaded
    spawn_ui(&mut commands, handle_color, handle_gray);
}

/// helper function to convert an image to grayscale by modifying its pixel data on the CPU
fn image_to_grayscale(image: &Image) -> Image {
    // Convert to a predictable 4-byte-per-pixel format first.
    let mut img = image
        .clone()
        .convert(TextureFormat::Rgba8UnormSrgb)
        .expect("failed to convert image to RGBA8");

    let data = img.data.as_mut().expect("image has no CPU-side pixel data");

    // RGBA8 pixel layout: [r, g, b, a, r, g, b, a, ...]
    for px in data.chunks_exact_mut(4) {
        let r = px[0] as f32;
        let g = px[1] as f32;
        let b = px[2] as f32;

        // Perceptual luma (BT.709-ish)
        let gray = (0.2126 * r + 0.7152 * g + 0.0722 * b)
            .round()
            .clamp(0.0, 255.0) as u8;

        px[0] = gray;
        px[1] = gray;
        px[2] = gray;
        // px[3] (alpha) preserved
    }

    img
}

fn spawn_ui(commands: &mut Commands, handle_color: Handle<Image>, handle_gray: Handle<Image>) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|root| {
            [("Color", handle_color), ("Gray", handle_gray)]
                .into_iter()
                .for_each(|(label, image)| {
                    root.spawn((Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },))
                        .with_children(|col| {
                            col.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                            ));
                            col.spawn((
                                ImageNode::new(image),
                                Node {
                                    width: Val::Px(512.0),
                                    height: Val::Px(512.0),
                                    ..default()
                                },
                            ));
                        });
                })
        });
}
