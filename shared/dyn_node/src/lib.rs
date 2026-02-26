use bevy::prelude::*;
use derive_more::From;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;

pub mod prelude {
    pub use super::{DynNode, DynNodePlugin};
}

/// make sure to register after any other asset loaders that are more sepcific than this before this (e.g. ".enemy.yml")
pub struct DynNodePlugin;

impl Plugin for DynNodePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DynNode>();

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

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Path not found: {0}")]
    NotFound(String),
    #[error("Failed to deserialize value at path: {0} into type: {1}")]
    DeserializeError(String, String),
}

/// Generic Config Node format (can be deserialized from json, yaml, ...)
#[derive(Deserialize, Asset, TypePath, From, Clone)]
pub struct DynNode(Value);

impl DynNode {
    /// query a config file at a given path. using json pointer format (starts with /)
    pub fn query_raw(&self, path: &str) -> Result<&Value, QueryError> {
        self.0
            .pointer(path)
            .ok_or_else(|| QueryError::NotFound(path.to_string()))
    }

    /// query a config file at a given path and deserialize it. using json pointer format (starts with /)
    pub fn query<T: DeserializeOwned>(&self, path: &str) -> Result<T, QueryError> {
        let raw = self.query_raw(path)?;
        serde_json::from_value(raw.clone())
            .map_err(|e| QueryError::DeserializeError(path.to_string(), e.to_string()))
    }
}
