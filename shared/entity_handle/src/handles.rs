use super::*;

pub struct EntityHandle<Intent = ()> {
    entity: Entity,
    entity_handle: Arc<StrongEntityHandle>,
    intent_handle: Arc<StrongEntityIntentHandle>,
    _marker: PhantomData<Intent>,
}

pub struct EntityAssetHandle<Id: IdTrait, Intent = ()> {
    handle: EntityHandle<Intent>,
    _marker: PhantomData<Id>,
}

impl<Intent> EntityHandle<Intent> {
    pub(crate) fn new(
        entity: Entity,
        entity_handle: Arc<StrongEntityHandle>,
        intent_handle: Arc<StrongEntityIntentHandle>,
    ) -> Self {
        Self {
            entity,
            entity_handle,
            intent_handle,
            _marker: PhantomData,
        }
    }
}

impl<Id: IdTrait, Intent> EntityAssetHandle<Id, Intent> {
    pub(crate) unsafe fn upcast_unchecked(handle: EntityHandle<Intent>) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }
}

impl<Intent> Deref for EntityHandle<Intent> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl<Id: IdTrait, Intent> Deref for EntityAssetHandle<Id, Intent> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.handle.entity
    }
}

impl<Intent> Clone for EntityHandle<Intent> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            entity_handle: Arc::clone(&self.entity_handle),
            intent_handle: Arc::clone(&self.intent_handle),
            _marker: PhantomData,
        }
    }
}

impl<Id: IdTrait, Intent> Clone for EntityAssetHandle<Id, Intent> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            _marker: PhantomData,
        }
    }
}

impl<Id: IdTrait, Intent> EntityAssetHandle<Id, Intent> {
    pub fn to_untyped(&self) -> EntityHandle<Intent> {
        self.handle.clone()
    }
}

impl<Intent> std::fmt::Debug for EntityHandle<Intent> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EntityHandle").field(&self.entity).finish()
    }
}

impl<Id: IdTrait, Intent> std::fmt::Debug for EntityAssetHandle<Id, Intent> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EntityAssetHandle")
            .field(&self.handle.entity)
            .finish()
    }
}
