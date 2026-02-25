use std::{any::TypeId, borrow::Borrow};

use bevy::{
    asset::AssetHandleProvider, ecs::system::SystemParam, platform::collections::HashMap,
    prelude::*, utils::TypeIdMap,
};
use derive_more::From;
use on_asset_loaded::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

/// make sure to register after any other asset loaders that are more sepcific than this before this (e.g. ".enemy.yml")
pub struct DynNodePlugin;

pub mod prelude {
    pub use super::{AppExt as _, DynNode, DynNodePlugin, DynNodeResolver};
}

impl Plugin for DynNodePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HandleProviders>()
            .init_resource::<HandleRegistry>()
            .init_dyn_asset::<DynNode>();

        #[cfg(feature = "cbor")]
        {
            use bevy_common_assets::cbor::CborAssetPlugin;
            app.add_plugins(CborAssetPlugin::<DynNode>::new(&["cbor"]));
        }
        #[cfg(feature = "json")]
        {
            use bevy_common_assets::json::JsonAssetPlugin;
            app.add_plugins(JsonAssetPlugin::<DynNode>::new(&["json", "jsonc"]));
        }
        #[cfg(feature = "ron")]
        {
            use bevy_common_assets::ron::RonAssetPlugin;
            app.add_plugins(RonAssetPlugin::<DynNode>::new(&["ron"]));
        }
        #[cfg(feature = "toml")]
        {
            use bevy_common_assets::toml::TomlAssetPlugin;
            app.add_plugins(TomlAssetPlugin::<DynNode>::new(&["toml"]));
        }
        #[cfg(feature = "yaml")]
        {
            use bevy_common_assets::yaml::YamlAssetPlugin;
            app.add_plugins(YamlAssetPlugin::<DynNode>::new(&["yml", "yaml"]));
        }
    }
}

pub trait AppExt {
    /// initialize an asset so it can be deserialized from a [`DynNode`] via a [`DynNodeResolver`].
    fn init_dyn_asset<A: Asset + DeserializeOwned>(&mut self) -> &mut Self;
}

impl AppExt for App {
    fn init_dyn_asset<A: Asset + DeserializeOwned>(&mut self) -> &mut Self {
        self.init_asset::<A>();

        let hp = self.world().resource::<Assets<A>>().get_handle_provider();
        self.world_mut()
            .resource_mut::<HandleProviders>()
            .insert(TypeId::of::<A>(), hp);

        self
    }
}

/// Generic Config Node format (can be deserialized from json, yaml, ...)
#[derive(Deserialize, Asset, TypePath, From, Clone)]
pub struct DynNode(Value);

impl DynNode {
    /// query a config file at a given path. using json pointer format (starts with /)
    pub fn query_raw(&self, path: &str) -> Option<&Value> {
        self.0.pointer(path)
    }

    /// query a config file at a given path and deserialize it. using json pointer format (starts with /)
    pub fn query<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        self.query_raw(path)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
}

#[derive(SystemParam)]
pub struct DynNodeResolver<'w, 's> {
    registry: ResMut<'w, HandleRegistry>,
    commands: Commands<'w, 's>,
    handle_providers: Res<'w, HandleProviders>,
}

impl DynNodeResolver<'_, '_> {
    pub fn resolve<A: Asset + DeserializeOwned>(
        &mut self,
        h_config: impl Borrow<Handle<DynNode>>,
        query: String,
    ) -> Handle<A> {
        let asset_id = h_config.borrow().id();
        let type_id = TypeId::of::<A>();

        if let Some(handle) = self.registry.0.get(&(asset_id, query.clone(), type_id)) {
            return handle.clone().typed();
        }

        let handle = self
            .handle_providers
            .get(&type_id)
            .expect("ExtractFromConfigPlugin not added for this asset type")
            .reserve_handle()
            .typed();

        self.commands.on_loaded_with(
            h_config.borrow(),
            (handle.clone(), query),
            on_loaded_query_extract::<A>,
        );

        handle
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct HandleProviders(TypeIdMap<AssetHandleProvider>);

#[derive(Resource, Default, Deref, DerefMut)]
struct HandleRegistry(HashMap<(AssetId<DynNode>, String, TypeId), UntypedHandle>);

/// On Asset Loaded Processor that queries a config node for a path and deserializes the result as an asset
/// Example:
///
/// ```
/// let h_config: Handle<ConfigNode> = asset_server.load("config.yml");
/// let h_enemy: Handle<Enemy> = enemies.reserve_handle();
/// commands.on_loaded_with(
///     &h_config,
///     (h_enemy.clone(), "/enemies/aligator".to_string()),
///     query_config_processor::<Enemy>,
/// );
/// ```
///
/// given config.yml:
/// ```yaml
/// enemies:
///   aligator:
///     health: 100
///     damage: 10
/// ```
///
fn on_loaded_query_extract<A: DeserializeOwned + Asset>(
    input: OnLoaded<DynNode, (Handle<A>, String)>,
    mut assets: ResMut<Assets<A>>,
) {
    let config = input.asset;
    let out = input.params.0;
    let path = input.params.1;

    let Some(a) = config.query(&path) else {
        panic!("Failed to query config for path: {path}");
    };
    assets.insert(out.id(), a).expect("Failed to insert asset");
}
