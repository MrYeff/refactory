use bevy::{
    ecs::{
        query::{QueryData, QueryEntityError, QueryFilter, ROQueryItem},
        system::SystemParam,
    },
    platform::collections::HashMap,
    prelude::*,
};
use std::fmt::Debug;
use std::hash::Hash;
use thiserror::Error;

pub mod prelude {
    pub use crate::{AddRegistryExt as _, Registry, RegistryQuery, RegistryQueryError};
}

#[derive(Resource)]
pub struct Registry<T: Component + Hash + Eq + Clone + Debug>(HashMap<T, Entity>);

impl<T: Component + Hash + Eq + Clone + Debug> Registry<T> {
    pub fn get(&self, component: &T) -> Option<&Entity> {
        self.0.get(component)
    }

    fn on_component_inserted(trigger: On<Insert, T>, mut registry: ResMut<Self>, val: Query<&T>) {
        let entity = trigger.entity;
        let Ok(component) = val.get(entity) else {
            return;
        };

        if registry.0.try_insert(component.clone(), entity).is_err() {
            panic!("Duplicate component value inserted into registry: {component:?}");
        }
    }

    fn on_component_removed(trigger: On<Remove, T>, mut registry: ResMut<Self>, val: Query<&T>) {
        let entity = trigger.entity;
        let Ok(component) = val.get(entity) else {
            return;
        };
        registry.0.remove(component);
    }

    fn plugin(app: &mut App) {
        app.insert_resource(Self(HashMap::default()))
            .add_observer(Self::on_component_inserted)
            .add_observer(Self::on_component_removed);
    }
}

pub trait AddRegistryExt {
    fn make_registry<T: Component + Hash + Eq + Clone + Debug>(&mut self) -> &mut Self;
}

impl AddRegistryExt for App {
    fn make_registry<T: Component + Hash + Eq + Clone + Debug>(&mut self) -> &mut Self {
        self.add_plugins(Registry::<T>::plugin)
    }
}

#[derive(Debug, Error)]
pub enum RegistryQueryError {
    #[error("No such entity exists for the given component")]
    NoSuchEntity,
    #[error("query error: {0}")]
    QueryEntityError(QueryEntityError),
}

#[derive(SystemParam, Deref, DerefMut)]
pub struct RegistryQuery<'w, 's, T, D, F = ()>
where
    T: Component + Hash + Eq + Clone + Debug,
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    #[deref]
    query: Query<'w, 's, D, (With<T>, F)>,
    registry: Res<'w, Registry<T>>,
}

impl<'w, 's, T, D, F> RegistryQuery<'w, 's, T, D, F>
where
    T: Component + Hash + Eq + Clone + Debug,
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    pub fn get(&self, key: &T) -> Result<ROQueryItem<'_, 's, D>, RegistryQueryError> {
        let entity = self
            .registry
            .get(key)
            .ok_or(RegistryQueryError::NoSuchEntity)?;

        self.query
            .get(*entity)
            .map_err(RegistryQueryError::QueryEntityError)
    }

    pub fn get_mut(&mut self, key: &T) -> Result<D::Item<'_, 's>, RegistryQueryError> {
        let entity = self
            .registry
            .get(key)
            .ok_or(RegistryQueryError::NoSuchEntity)?;

        self.query
            .get_mut(*entity)
            .map_err(RegistryQueryError::QueryEntityError)
    }
}
