use async_channel::{Receiver, Sender};
use bevy::{ecs::system::SystemId, prelude::*};
use std::iter;

pub struct AsyncServicePlugin;

impl Plugin for AsyncServicePlugin {
    fn build(&self, app: &mut App) {
        let (service, service_rx) = new_async_service();
        app.insert_resource(service)
            .insert_resource(service_rx)
            .init_resource::<Processors>()
            .add_systems(Update, execute_processors);
    }
}

#[derive(Resource, Clone)]
pub struct AsyncService {
    registration: Sender<Box<dyn FnOnce(&mut World) + Send + Sync>>,
}

impl AsyncService {
    /// Can be called inside an async system to execute this as a sync system
    pub async fn exec_sync<I, O, M, S>(&self, system: S, input: I) -> O
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<In<I>, O, M> + Send + Sync + 'static,
    {
        let (tx, rx) = async_channel::bounded::<(SystemId<In<I>, O>, SystemBridgeAsync<I, O>)>(1);
        self.registration
            .send(Box::new(move |world| {
                let sys = world.register_system_cached(system);
                if !world.is_resource_added::<BridgeMarker<I, O>>() {
                    world.init_resource::<BridgeMarker<I, O>>();
                    world
                        .resource_mut::<Processors>()
                        .0
                        .push(Box::new(try_exec_processor::<I, O>));
                    let req = async_channel::unbounded::<(SystemId<In<I>, O>, I)>();
                    let rsp = async_channel::unbounded::<O>();
                    let bridge_async = SystemBridgeAsync {
                        req: req.0,
                        rsp: rsp.1,
                    };
                    world.insert_resource(bridge_async.clone());
                    world.insert_resource(SystemBridgeSync {
                        req: req.1,
                        rsp: rsp.0,
                    });
                    tx.send_blocking((sys, bridge_async)).unwrap();
                } else {
                    tx.send_blocking((sys, world.resource::<SystemBridgeAsync<I, O>>().clone()))
                        .unwrap();
                }
            }))
            .await
            .unwrap();

        let (sys, bridge) = rx.recv().await.unwrap();
        bridge.req.send((sys, input)).await.unwrap();
        bridge.rsp.recv().await.unwrap()
    }
}

#[derive(Resource)]
struct AsyncServiceRx {
    registration: Receiver<Box<dyn FnOnce(&mut World) + Send + Sync>>,
}

fn new_async_service() -> (AsyncService, AsyncServiceRx) {
    let (tx, rx) = async_channel::unbounded::<Box<dyn FnOnce(&mut World) + Send + Sync>>();
    (
        AsyncService { registration: tx },
        AsyncServiceRx { registration: rx },
    )
}

#[derive(Resource)]
struct BridgeMarker<I, O> {
    _marker: std::marker::PhantomData<(I, O)>,
}

impl<I, O> Default for BridgeMarker<I, O> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

/// Async Side
#[derive(Resource)]
struct SystemBridgeAsync<I: 'static, O> {
    req: Sender<(SystemId<In<I>, O>, I)>,
    rsp: Receiver<O>,
}

impl<I: 'static, O> Clone for SystemBridgeAsync<I, O> {
    fn clone(&self) -> Self {
        Self {
            req: self.req.clone(),
            rsp: self.rsp.clone(),
        }
    }
}

/// Sync Sid
#[derive(Resource)]
struct SystemBridgeSync<I: 'static, O> {
    req: Receiver<(SystemId<In<I>, O>, I)>,
    rsp: Sender<O>,
}

fn try_exec_processor<In, Out>(world: &mut World)
where
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    world.resource_scope::<SystemBridgeSync<In, Out>, _>(|world, bridge| {
        while let Ok((sys, input)) = bridge.req.try_recv() {
            let output = world.run_system_with(sys, input).unwrap();
            bridge.rsp.send_blocking(output).unwrap();
        }
    });
}

#[derive(Resource, Default, Deref)]
struct Processors(Vec<Box<dyn Fn(&mut World) + Send + Sync>>);

fn execute_processors(world: &mut World) {
    world.resource_scope::<AsyncServiceRx, _>(|world, async_service| {
        iter::from_fn(|| async_service.registration.try_recv().ok()).for_each(|reg| reg(world));
    });

    world.resource_scope::<Processors, _>(|world, processors| {
        for proc in processors.0.iter() {
            proc(world);
        }
    })
}
