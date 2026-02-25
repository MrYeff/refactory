mod entries;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use dyn_node::prelude::*;
use itertools::Itertools as _;
use on_asset_loaded::prelude::*;
use registry::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_with::serde_as;
use std::sync::Arc;
use std::time::Duration;

pub struct AtlasPlugin;

impl Plugin for AtlasPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DynNodePlugin, AssetObserverPlugin))
            .make_registry::<AtlasEntryId>()
            .init_dyn_asset::<AtlasEntryDefinition>();
    }
}

#[derive(Debug, Clone, Component, Hash, PartialEq, Eq)]
#[component(immutable)]
pub struct AtlasHandle(pub Handle<DynNode>);

#[derive(Debug, Clone, Deserialize)]
struct FrameSequence {
    stride: UVec2,
    count: u32,
}

impl GetFrameIndex for FrameSequence {
    type Param = u32;
    fn get_frame_index(&self, frame: u32) -> UVec2 {
        let frame = frame % self.count;
        UVec2::new(self.stride.x * frame, self.stride.y * frame)
    }
}

pub trait PathSuffix {
    const SUFFIX: &'static str;
}

pub trait GetFrameIndex {
    type Param;
    fn get_frame_index(&self, param: Self::Param) -> UVec2;
}

#[derive(Component)]
#[relationship(relationship_target=Entries)]
pub struct Atlas(Entity);

#[derive(Component)]
#[relationship_target(relationship=Atlas)]
#[component(on_remove=Self::on_remove)]
pub struct Entries(Vec<Entity>);

impl Entries {
    /// remove when not referenced anymore
    fn on_remove(mut world: DeferredWorld, ctx: HookContext) {
        world.commands().entity(ctx.entity).despawn();
    }
}

pub mod atlas_entry {
    use super::*;

    pub(crate) fn load_atlas_entry_component<
        A: Asset + Component + Clone + DeserializeOwned + PathSuffix,
    >(
        to_load: Query<
            (Entity, &AtlasHandle, &AtlasEntryId),
            (Without<A>, Without<AddWhenLoaded<A>>),
        >,
        mut resolver: DynNodeResolver,
        mut commands: Commands,
    ) {
        to_load
            .iter()
            .for_each(|(entity, AtlasHandle(h_atlas), AtlasEntryId(id))| {
                let h_entry = resolver.resolve::<A>(
                    h_atlas.clone(),
                    format!("/entries/{id}{}", A::SUFFIX).to_string(),
                );
                commands
                    .entity(entity)
                    .insert(AddWhenLoadedBundle::new(h_entry));
            });
    }

    #[derive(Debug, Clone, Component, Hash, PartialEq, Eq)]
    #[component(immutable)]
    pub struct AtlasEntryId(pub Arc<str>);

    pub mod entries {
        use super::*;

        #[derive(Debug, Clone, Component, Deserialize, Asset, TypePath)]
        pub struct AtlasEntryDefinition {
            pub size: UVec2,
            pub offset: UVec2,
        }

        impl GetFrameIndex for AtlasEntryDefinition {
            type Param = ();
            fn get_frame_index(&self, _: Self::Param) -> UVec2 {
                self.offset
            }
        }

        #[serde_as]
        #[derive(Asset, Debug, Clone, Component, TypePath, Deserialize)]
        pub struct AnimationDefinition {
            #[serde_as(as = "serde_with::DurationSeconds<u64>")]
            frame_duration: Duration,
            seq: FrameSequence,
        }

        impl GetFrameIndex for AnimationDefinition {
            type Param = Duration;
            fn get_frame_index(&self, total_time: Self::Param) -> UVec2 {
                let frame =
                    (total_time.as_secs_f32() / self.frame_duration.as_secs_f32()).floor() as u32;
                self.seq.get_frame_index(frame)
            }
        }

        #[derive(Asset, Debug, Clone, Component, TypePath, Deserialize)]
        pub struct VariantsDefinition {
            seq: FrameSequence,
        }

        impl GetFrameIndex for VariantsDefinition {
            type Param = u32;
            fn get_frame_index(&self, variant: Self::Param) -> UVec2 {
                self.seq.get_frame_index(variant)
            }
        }

        #[derive(Asset, Debug, Clone, Component, TypePath, Deserialize)]
        pub struct RotationsDefinition {
            rotation_count: GridRotations,
            seq: FrameSequence,
        }

        impl GetFrameIndex for RotationsDefinition {
            type Param = u32;
            fn get_frame_index(&self, rotation: Self::Param) -> UVec2 {
                let frame = match self.rotation_count {
                    GridRotations::PerAxis => rotation % 4 / 2,
                    GridRotations::All => rotation % 4,
                };
                self.seq.get_frame_index(frame)
            }
        }

        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum GridRotations {
            /// Expects 2 frames: one for 0°+180° and one for 90°+270°
            #[serde(rename = "per_axis")]
            PerAxis,
            /// Expects 4 frames: one for each 90° rotation
            #[serde(rename = "all")]
            All,
        }

        impl PathSuffix for AtlasEntryDefinition {
            const SUFFIX: &'static str = "";
        }

        impl PathSuffix for AnimationDefinition {
            const SUFFIX: &'static str = "/animation";
        }

        impl PathSuffix for VariantsDefinition {
            const SUFFIX: &'static str = "/variants";
        }

        impl PathSuffix for RotationsDefinition {
            const SUFFIX: &'static str = "/rotations";
        }
    }
}

use atlas_entry::entries::*;
use atlas_entry::*;

mod atlas {

    use bevy::render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureViewDescriptor,
        TextureViewDimension,
    };

    use super::*;

    pub(crate) fn load_atlas_component<A: Asset + Component + Clone + DeserializeOwned>(
        to_load: Query<
            (Entity, &AtlasHandle),
            (With<AtlasDefinition>, Without<A>, Without<AddWhenLoaded<A>>),
        >,
        mut resolver: DynNodeResolver,
        mut commands: Commands,
    ) {
        to_load.iter().for_each(|(entity, AtlasHandle(h_atlas))| {
            let h_entry = resolver.resolve::<A>(h_atlas.clone(), "".to_string());
            commands
                .entity(entity)
                .insert(AddWhenLoadedBundle::new(h_entry));
        });
    }

    #[derive(Asset, Debug, Clone, Component, TypePath, Deserialize)]
    pub struct AtlasDefinition {
        tile_size: UVec2,
    }

    #[derive(Debug, Clone, Component, TypePath, Deserialize)]
    #[component(immutable, on_add=Self::on_add)]
    pub struct SourceImagePath {
        path: String,
    }

    impl SourceImagePath {
        fn on_add(mut world: DeferredWorld, ctx: HookContext) {
            let entity = ctx.entity;
            let path = world
                .entity(entity)
                .get::<SourceImagePath>()
                .unwrap()
                .path
                .clone();
            let handle = world.resource::<AssetServer>().load(path);
            let mut commands = world.commands();

            commands.entity(entity).insert(RawImage(handle.clone()));
            commands.on_loaded_with(
                handle.clone(),
                entity,
                |input: OnLoaded<Image, Entity>,
                mut commands: Commands,
                defs: Query<&AtlasDefinition>,
                mut images: ResMut<Assets<Image>>| {
                    let def = defs.get(input.params).unwrap(); // probably should be handled better

                    let (image, original_tile_width) =
                        tileset_to_stacked(&input.asset, def.tile_size);

                    let h_stack = images.add(image);

                    commands.entity(input.params).insert(StackedImage {
                        handle: h_stack,
                        original_tile_width,
                    });
                },
            );
        }
    }

    #[derive(Debug, Clone, Component)]
    pub struct RawImage(Handle<Image>);

    #[derive(Debug, Clone, Component)]
    pub struct StackedImage {
        handle: Handle<Image>,
        original_tile_width: u32,
    }

    impl StackedImage {
        pub fn offste_to_idx(&self, offset: UVec2) -> UVec2 {
            UVec2::new(offset.x / self.original_tile_width, offset.y)
        }
    }

    fn tileset_to_stacked(image: &Image, tile_size: UVec2) -> (Image, u32) {
        assert_eq!(image.texture_descriptor.dimension, TextureDimension::D2);
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 1);
        assert_eq!(image.height() % tile_size.y, 0);
        assert_eq!(image.width() % tile_size.x, 0);

        let sheet_w = image.texture_descriptor.size.width;
        let sheet_h = image.texture_descriptor.size.height;

        let tiles_w = sheet_w / tile_size.x;
        let tiles_h = sheet_h / tile_size.y;

        let bpp = bytes_per_pixel(image.texture_descriptor.format).unwrap();
        let data = (&image.data).as_ref().unwrap();

        let mut data_new = Vec::with_capacity(data.len());

        (0..tiles_h)
            .cartesian_product(0..tiles_w)
            .for_each(|(ty, tx)| {
                let src_x0 = tx * tile_size.x;
                let src_y0 = ty * tile_size.y;

                (0..tile_size.y).for_each(|by| {
                    let src_px = (src_y0 + by) * sheet_w + src_x0;
                    let byte_offset = src_px as usize * bpp;
                    let byte_w = (tile_size.x as usize) * bpp;

                    data_new.extend_from_slice(&data[byte_offset..byte_offset + byte_w]);
                });
            });

        (
            Image {
                data: Some(data_new),
                texture_descriptor: TextureDescriptor {
                    size: Extent3d {
                        width: tile_size.x,
                        height: tile_size.y,
                        depth_or_array_layers: tiles_w * tiles_h,
                    },
                    ..image.texture_descriptor
                },
                texture_view_descriptor: Some(TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2Array),
                    ..default()
                }),
                ..image.clone()
            },
            tiles_h,
        )
    }

    fn bytes_per_pixel(format: TextureFormat) -> Option<usize> {
        use TextureFormat::*;
        Some(match format {
            R8Unorm | R8Snorm | R8Uint | R8Sint => 1,
            Rg8Unorm | Rg8Snorm | Rg8Uint | Rg8Sint => 2,
            Rgba8Unorm | Rgba8UnormSrgb | Rgba8Snorm | Rgba8Uint | Rgba8Sint | Bgra8Unorm
            | Bgra8UnormSrgb => 4,
            Rgba16Float | Rgba16Unorm | Rgba16Snorm | Rgba16Uint | Rgba16Sint => 8,
            Rgba32Float | Rgba32Uint | Rgba32Sint => 16,
            _ => return None,
        })
    }
}
