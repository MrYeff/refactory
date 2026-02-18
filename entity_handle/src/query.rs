use super::*;

#[derive(SystemParam)]
pub struct EntityAssetQuery<'w, 's, Id, D, F = ()>
where
    Id: IdTrait,
    D: 'static + QueryData,
    F: 'static + QueryFilter,
{
    query: Query<'w, 's, D, (With<IdMarker<Id>>, F)>,
}

impl<'w, 's, Id, D, F> EntityAssetQuery<'w, 's, Id, D, F>
where
    Id: IdTrait,
    D: QueryData,
    F: QueryFilter,
{
    pub fn get<Intent>(
        &self,
        handle: &EntityAssetHandle<Id, Intent>,
    ) -> Result<ROQueryItem<'_, 's, D>, QueryEntityError> {
        self.query.get(**handle)
    }

    pub fn get_mut<Intent>(
        &mut self,
        handle: &EntityAssetHandle<Id, Intent>,
    ) -> Result<D::Item<'_, 's>, QueryEntityError> {
        self.query.get_mut(**handle)
    }

    pub fn iter(&self) -> impl Iterator<Item = ROQueryItem<'_, 's, D>> + '_ {
        self.query.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = D::Item<'_, 's>> + '_ {
        self.query.iter_mut()
    }
}
