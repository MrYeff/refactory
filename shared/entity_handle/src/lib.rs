#![feature(trait_alias)]
#![feature(arbitrary_self_types)]
#![feature(result_option_map_or_default)]

pub mod handles;
pub mod markers;
pub mod query;

use bevy::{
    ecs::{
        component::ComponentId,
        query::{QueryData, QueryEntityError, QueryFilter, ROQueryItem},
        system::SystemParam,
    },
    platform::collections::HashMap,
    prelude::*,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::{
    any::TypeId,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Weak},
};

pub trait ThreadSafe = Send + Sync + 'static;
pub trait IdTrait = ThreadSafe + Hash + Eq + Clone;
use handles::*;
use markers::*;

pub mod prelude {
    pub use crate::handles::{EntityAssetHandle, EntityHandle};
    pub use crate::markers::IntentMarker;
    pub use crate::query::EntityAssetQuery;
    pub use crate::{EntityAssetServer, EntityHandlePlugin, EntityServer, ExtEntityHandle};
}

pub struct EntityHandlePlugin;

impl Plugin for EntityHandlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityHandler>()
            .add_systems(PostUpdate, EntityHandler::execute);

        app.init_resource::<IntentComponentsRegistry>();

        app.register_intent::<()>();
    }
}

#[derive(Clone, Copy)]
struct DropEntityEvent(Entity);

#[derive(Clone, Copy)]
struct DropIntentEvent {
    entity: Entity,
    intent_component: ComponentId,
}

struct StrongEntityHandle {
    entity: Entity,
    drop_queue: Sender<DropEntityEvent>,
}

struct StrongEntityIntentHandle {
    entity: Entity,
    intent_component: ComponentId,
    drop_queue: Sender<DropIntentEvent>,
}

impl Drop for StrongEntityHandle {
    fn drop(&mut self) {
        let _ = self.drop_queue.send(DropEntityEvent(self.entity));
    }
}

impl Drop for StrongEntityIntentHandle {
    fn drop(&mut self) {
        let _ = self.drop_queue.send(DropIntentEvent {
            entity: self.entity,
            intent_component: self.intent_component,
        });
    }
}

#[derive(Resource)]
struct EntityHandler {
    drop_entity_rx: Receiver<DropEntityEvent>,
    drop_intent_rx: Receiver<DropIntentEvent>,
    drop_entity_tx: Sender<DropEntityEvent>,
    drop_intent_tx: Sender<DropIntentEvent>,
    entity_handles: HashMap<Entity, Weak<StrongEntityHandle>>,
    intent_handles: HashMap<(Entity, ComponentId), Weak<StrongEntityIntentHandle>>,
}

impl EntityHandler {
    /// applies tthe drop events
    fn execute(mut self: ResMut<Self>, mut commands: Commands) {
        while let Ok(event) = self.drop_intent_rx.try_recv() {
            // only remove droped handles (maybe got set again by regaining handle)
            if self
                .intent_handles
                .get(&(event.entity, event.intent_component))
                .map_or_default(|h| h.upgrade().is_none())
            {
                commands
                    .entity(event.entity)
                    .remove_by_id(event.intent_component);

                self.intent_handles
                    .remove(&(event.entity, event.intent_component));
            }
        }

        while let Ok(event) = self.drop_entity_rx.try_recv() {
            // only remove droped handles (maybe got set again by regaining handle)
            if self
                .entity_handles
                .get(&event.0)
                .map_or_default(|h| h.upgrade().is_none())
            {
                commands.entity(event.0).despawn();
                self.entity_handles.remove(&event.0);
            }
        }
    }
}

impl Default for EntityHandler {
    fn default() -> Self {
        let (drop_entity_tx, drop_entity_rx) = unbounded();
        let (drop_intent_tx, drop_intent_rx) = unbounded();

        Self {
            drop_entity_rx,
            drop_intent_rx,
            drop_entity_tx,
            drop_intent_tx,
            entity_handles: HashMap::new(),
            intent_handles: HashMap::new(),
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
struct EntityRegistry<Id: IdTrait>(HashMap<Id, Entity>);

impl<Id: IdTrait> Default for EntityRegistry<Id> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
struct IntentComponentsRegistry(HashMap<TypeId, ComponentId>);

pub trait ExtEntityHandle {
    fn register_entity_asset_id<Id: IdTrait>(&mut self);
    fn register_intent<Intent: ThreadSafe>(&mut self);
}

impl ExtEntityHandle for App {
    fn register_entity_asset_id<Id: IdTrait>(&mut self) {
        self.init_resource::<EntityRegistry<Id>>();
    }

    fn register_intent<Intent: ThreadSafe>(&mut self) {
        let intent_type_id = TypeId::of::<Intent>();
        let intent_component_id = self
            .world_mut()
            .register_component::<IntentMarker<Intent>>();

        self.world_mut()
            .get_resource_mut::<IntentComponentsRegistry>()
            .expect("register entity_handle plugin before registering intents")
            .insert(intent_type_id, intent_component_id);
    }
}

#[derive(SystemParam)]
pub struct EntityServer<'w, 's> {
    handler: ResMut<'w, EntityHandler>,
    intent_registry: Res<'w, IntentComponentsRegistry>,
    commands: Commands<'w, 's>,
}

#[derive(SystemParam, Deref)]
pub struct EntityAssetServer<'w, 's, Id: IdTrait> {
    #[deref]
    server: EntityServer<'w, 's>,
    registry: ResMut<'w, EntityRegistry<Id>>,
}

impl<'w, 's> EntityServer<'w, 's> {
    /// Spawn a new empty entity to be managed by the handler, returns a handle to it. The entity will be despawned when all handles to it are dropped.
    pub fn spawn_empty<Intent: ThreadSafe>(&mut self) -> EntityHandle<Intent> {
        self.spawn(())
    }

    /// Spawn a new entity to be managed by the handler, returns a handle to it. The entity will be despawned when all handles to it are dropped.
    pub fn spawn<Intent: ThreadSafe>(&mut self, bundle: impl Bundle) -> EntityHandle<Intent> {
        let entity = self
            .commands
            .spawn((bundle, IntentMarker::<Intent>::new()))
            .id();

        let entity_handle = Arc::new(StrongEntityHandle {
            entity,
            drop_queue: self.handler.drop_entity_tx.clone(),
        });

        let intent_component = *self
            .intent_registry
            .get(&TypeId::of::<Intent>())
            .expect("register intent before spawning entities with it");

        let intent_handle = Arc::new(StrongEntityIntentHandle {
            entity,
            intent_component,
            drop_queue: self.handler.drop_intent_tx.clone(),
        });

        self.handler
            .entity_handles
            .insert(entity, Arc::downgrade(&entity_handle));

        self.handler
            .intent_handles
            .insert((entity, intent_component), Arc::downgrade(&intent_handle));

        EntityHandle::new(entity, entity_handle, intent_handle)
    }

    /// Converts an existing entity to be managed by the handler, returns a handle to it. The entity will be despawned when all handles to it are dropped.
    /// If the entity is already managed by the handler, returns a handle to it.
    pub fn to_managed<Intent: ThreadSafe>(&mut self, entity: Entity) -> EntityHandle<Intent> {
        let entity_handle = match self
            .handler
            .entity_handles
            .get(&entity)
            .and_then(|h| h.upgrade())
        {
            Some(handle) => handle.clone(),
            None => {
                let new_handle = Arc::new(StrongEntityHandle {
                    entity,
                    drop_queue: self.handler.drop_entity_tx.clone(),
                });

                self.handler
                    .entity_handles
                    .insert(entity, Arc::downgrade(&new_handle));

                new_handle
            }
        };
        let intent_component = *self
            .intent_registry
            .get(&TypeId::of::<Intent>())
            .expect("register intent before spawning entities with it");

        let intent_handle = match self
            .handler
            .intent_handles
            .get(&(entity, intent_component))
            .and_then(|h| h.upgrade())
        {
            Some(handle) => handle.clone(),
            None => {
                let new_handle = Arc::new(StrongEntityIntentHandle {
                    entity,
                    intent_component,
                    drop_queue: self.handler.drop_intent_tx.clone(),
                });

                self.handler.intent_handles.insert(
                    (entity, new_handle.intent_component),
                    Arc::downgrade(&new_handle),
                );

                self.commands
                    .entity(entity)
                    .insert(IntentMarker::<Intent>::new());

                new_handle
            }
        };

        EntityHandle::new(entity, entity_handle, intent_handle)
    }
}

impl<'w, 's, Id: IdTrait> EntityAssetServer<'w, 's, Id> {
    pub fn get_asset<Intent: ThreadSafe>(&mut self, id: Id) -> EntityAssetHandle<Id, Intent> {
        if let Some(entity) = self.registry.get(&id) {
            let handle = self.server.to_managed::<Intent>(*entity);
            unsafe {
                // SAFE: the entity is already registered under they key type so this cast is valid
                EntityAssetHandle::upcast_unchecked(handle)
            }
        } else {
            let handle = self.server.spawn::<Intent>(IdMarker::<Id>::new());

            self.registry.insert(id, *handle);
            unsafe {
                // SAFE: the entity was just spawned with this component
                EntityAssetHandle::upcast_unchecked(handle)
            }
        }
    }
}
