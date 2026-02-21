use bevy_app::{App, Plugin};
use bevy_asset::io::Reader;
use bevy_asset::{Asset, AssetApp, AssetLoader, LoadContext};
use bevy_ecs::resource::Resource;
use bevy_reflect::TypePath;
use serde_yaml::Value as YamlValue;
use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};
use thiserror::Error;

/// Root asset for YAML files. (The value is in labeled subassets.)
#[derive(Asset, TypePath, Default)]
pub struct YamlFile;

/// Extractor signature: given the YAML node at the requested label, maybe emit a typed labeled asset.
type ExtractorFn = for<'a> fn(&YamlValue, &str, &mut LoadContext<'a>);

#[derive(Clone, Default, Resource)]
pub struct YamlExtractorRegistry {
    inner: Arc<RwLock<Vec<ExtractorFn>>>,
}

/// Store extensions so the loader plugin can be a unit type and still know which extensions to claim.
#[derive(Clone, Resource)]
struct YamlExtensions(Vec<&'static str>);

/// Single-loader plugin (unit type so `is_plugin_added` works).
pub struct AdvancedYamlLoaderPlugin;

impl Plugin for AdvancedYamlLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<YamlFile>()
            .init_resource::<YamlExtractorRegistry>();

        let exts = app.world().resource::<YamlExtensions>().0.clone();
        let registry = app.world().resource::<YamlExtractorRegistry>().clone();

        app.register_asset_loader(AdvancedYamlAssetLoader {
            extensions: exts,
            registry,
        });
    }
}

/// Plugin users add per type `A`.
/// This does NOT register a separate loader per `A`.
pub struct AdvancedYamlAssetPlugin<A> {
    extensions: Vec<&'static str>,
    _marker: PhantomData<A>,
}

impl<A> AdvancedYamlAssetPlugin<A> {
    pub fn new(extensions: &[&'static str]) -> Self {
        Self {
            extensions: extensions.to_owned(),
            _marker: PhantomData,
        }
    }
}

impl<A> Plugin for AdvancedYamlAssetPlugin<A>
where
    for<'de> A: serde::Deserialize<'de> + Asset + TypePath + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        // Ensure this asset type is known to Bevy.
        app.init_asset::<A>();

        // Ensure extensions are stored before loader is added.
        if !app.world().contains_resource::<YamlExtensions>() {
            app.insert_resource(YamlExtensions(self.extensions.clone()));
        }

        // Ensure the single loader is added once.
        if !app.is_plugin_added::<AdvancedYamlLoaderPlugin>() {
            app.add_plugins(AdvancedYamlLoaderPlugin);
        }

        // Register extractor for A.
        let registry = app.world().resource::<YamlExtractorRegistry>().clone();
        registry
            .inner
            .write()
            .expect("YamlExtractorRegistry poisoned")
            .push(extract_requested_node_as::<A>);
    }
}

/// Possible errors produced by the YAML loader.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum YamlLoaderError {
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("YAML path not found: {0}")]
    PathNotFound(String),
}

/// JSON Pointer unescaping: "~1" => "/", "~0" => "~"
fn unescape_token(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

fn navigate<'a>(root: &'a YamlValue, label: &str) -> Option<&'a YamlValue> {
    if label.is_empty() {
        return Some(root);
    }
    let mut cur = root;
    for raw in label.split('/') {
        let t = unescape_token(raw);
        cur = match cur {
            YamlValue::Mapping(map) => map.get(&YamlValue::String(t))?,
            YamlValue::Sequence(seq) => {
                let idx: usize = t.parse().ok()?;
                seq.get(idx)?
            }
            _ => return None,
        };
    }
    Some(cur)
}

/// Extract only the requested node into `A` and attach it as a labeled subasset.
/// This is called by the loader ONLY for the requested label (not for every node).
fn extract_requested_node_as<A>(node: &YamlValue, label: &str, load_context: &mut LoadContext<'_>)
where
    for<'de> A: serde::Deserialize<'de> + Asset,
{
    // If something already claimed this label, don't overwrite (first-wins).
    if load_context.has_labeled_asset(label.to_string()) {
        return;
    }

    if let Ok(asset) = serde_yaml::from_value::<A>(node.clone()) {
        load_context.add_labeled_asset(label.to_string(), asset);
    }
}

/// Single YAML loader. It parses the file and, if a label is present, only navigates that label.
/// It then asks registered extractors to emit typed labeled subassets for that label.
#[derive(TypePath)]
pub struct AdvancedYamlAssetLoader {
    extensions: Vec<&'static str>,
    registry: YamlExtractorRegistry,
}

impl AssetLoader for AdvancedYamlAssetLoader {
    type Asset = YamlFile;
    type Settings = ();
    type Error = YamlLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let root: YamlValue = serde_yaml::from_slice(&bytes)?;

        println!("LoadContext {:?}", load_context.path());

        // If no label: just load the root file asset.
        // (You can still choose to emit something like label "" here, but it's optional.)
        let Some(label) = load_context.path().label() else {
            return Ok(YamlFile::default());
        };
        let label = label.to_string();

        let node = navigate(&root, &label)
            .ok_or_else(|| YamlLoaderError::PathNotFound(label.to_string()))?;

        // Run extractors ONLY on this node + label.
        let extractors = self
            .registry
            .inner
            .read()
            .expect("YamlExtractorRegistry poisoned");

        for ex in extractors.iter().copied() {
            ex(node, &label, load_context);
        }

        Ok(YamlFile::default())
    }

    fn extensions(&self) -> &[&str] {
        &self.extensions
    }
}
