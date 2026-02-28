use super::*;

pub trait Spawner<P> {
    fn spawn(&self, commands: &mut Commands, params: P) -> Entity;
}

pub type BoxedSpawner<P> = Box<dyn Spawner<P> + Send + Sync>;

impl<P, B: Bundle, F: Fn(P) -> B> Spawner<P> for F {
    fn spawn(&self, commands: &mut Commands, params: P) -> Entity {
        commands.spawn(self(params)).id()
    }
}

pub trait IntoSpawner<P> {
    fn into_spawner(self) -> BoxedSpawner<P>;
}

impl<P, T: Spawner<P> + Send + Sync + 'static> IntoSpawner<P> for T {
    fn into_spawner(self) -> BoxedSpawner<P> {
        Box::new(self)
    }
}
