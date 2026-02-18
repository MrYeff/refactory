use super::*;

/// Component to identify entitites based on given id by the handler
#[derive(Component)]
pub(crate) struct IdMarker<Id: IdTrait>(pub(crate) PhantomData<Id>);

impl<Id: IdTrait> IdMarker<Id> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

/// Component to comunicate intent on entity state. primary integration point for external systems.
#[derive(Component)]
pub struct IntentMarker<Intent>(pub(crate) PhantomData<Intent>);

impl<Intent> IntentMarker<Intent> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}
