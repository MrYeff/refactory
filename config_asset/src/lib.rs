use bevy::prelude::*;
use derive_more::From;
use on_asset_loaded::OnLoaded;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

/// make sure to register after any other asset loaders that are more sepcific than this before this (e.g. ".enemy.yml")
pub struct ConfigAssetLoaderPlugin;

pub mod prelude {
    #[cfg(feature = "processor")]
    pub use super::on_loaded_query_extract;
    pub use super::{ConfigAssetLoaderPlugin, ConfigNode};
}

impl Plugin for ConfigAssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ConfigNode>();
        #[cfg(feature = "cbor")]
        {
            use bevy_common_assets::cbor::CborAssetPlugin;
            app.add_plugins(CborAssetPlugin::<ConfigNode>::new(&["cbor"]));
        }
        #[cfg(feature = "json")]
        {
            use bevy_common_assets::json::JsonAssetPlugin;
            app.add_plugins(JsonAssetPlugin::<ConfigNode>::new(&["json", "jsonc"]));
        }
        #[cfg(feature = "ron")]
        {
            use bevy_common_assets::ron::RonAssetPlugin;
            app.add_plugins(RonAssetPlugin::<ConfigNode>::new(&["ron"]));
        }
        #[cfg(feature = "toml")]
        {
            use bevy_common_assets::toml::TomlAssetPlugin;
            app.add_plugins(TomlAssetPlugin::<ConfigNode>::new(&["toml"]));
        }
        #[cfg(feature = "yaml")]
        {
            use bevy_common_assets::yaml::YamlAssetPlugin;
            app.add_plugins(YamlAssetPlugin::<ConfigNode>::new(&["yml", "yaml"]));
        }
    }
}

/// Generic Config Node format (can be deserialized from json, yaml, ...)
#[derive(Deserialize, Asset, TypePath, From, Clone)]
pub struct ConfigNode(Value);

impl ConfigNode {
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
#[cfg(feature = "processor")]
pub fn on_loaded_query_extract<A: DeserializeOwned + Asset>(
    input: OnLoaded<ConfigNode, (Handle<A>, String)>,
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
