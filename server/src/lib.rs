#![feature(drain_filter, bind_by_move_pattern_guards)]

pub mod auth_provider;
pub mod client;
pub mod cmd;
pub mod error;
pub mod input;
pub mod provider;
pub mod settings;

// Reexports
pub use crate::{error::Error, input::Input, settings::ServerSettings};

use crate::{
    auth_provider::AuthProvider,
    client::{Client, Clients},
    cmd::CHAT_COMMANDS,
};
use common::{
    comp,
    event::{Event as GameEvent, EventBus},
    msg::{ClientMsg, ClientState, RequestStateError, ServerError, ServerInfo, ServerMsg},
    net::PostOffice,
    state::{BlockChange, State, TimeOfDay, Uid, DirtiedChunks},
    terrain::{block::Block, TerrainChunk, TerrainChunkSize, TerrainMap},
    vol::Vox,
    vol::{ReadVol, VolSize},
};
use hashbrown::HashSet;
use log::debug;
use provider::Provider;
use rand::Rng;
use specs::{join::Join, world::EntityBuilder as EcsEntityBuilder, Builder, Entity as EcsEntity};
use std::ops::Deref;
use std::{
    i32,
    net::SocketAddr,
    sync::{mpsc, Arc},
    time::Duration,
};
use uvth::{ThreadPool, ThreadPoolBuilder};
use vek::*;
use world::{ChunkSupplement, World};

const CLIENT_TIMEOUT: f64 = 20.0; // Seconds

pub enum Event {
    ClientConnected {
        entity: EcsEntity,
    },
    ClientDisconnected {
        entity: EcsEntity,
    },
    Chat {
        entity: Option<EcsEntity>,
        msg: String,
    },
}

#[derive(Copy, Clone)]
struct SpawnPoint(Vec3<f32>);

pub struct Server {
    state: State,
    world_provider: Arc<Provider>,
    save_handle: Option<std::thread::JoinHandle<()>>,

    postoffice: PostOffice<ServerMsg, ClientMsg>,
    clients: Clients,

    thread_pool: ThreadPool,
    chunk_tx: mpsc::Sender<(Vec2<i32>, (TerrainChunk, ChunkSupplement))>,
    chunk_rx: mpsc::Receiver<(Vec2<i32>, (TerrainChunk, ChunkSupplement))>,
    pending_chunks: HashSet<Vec2<i32>>,

    server_settings: ServerSettings,
    server_info: ServerInfo,

    // TODO: anything but this
    accounts: AuthProvider,
}

impl Server {
    /// Create a new `Server` bound to the default socket.
    pub fn new(settings: ServerSettings) -> Result<Self, Error> {
        Self::bind(settings.address, settings)
    }

    /// Create a new server bound to the given socket.
    pub fn bind<A: Into<SocketAddr>>(addrs: A, settings: ServerSettings) -> Result<Self, Error> {
        let (chunk_tx, chunk_rx) = mpsc::channel();

        let mut state = State::default();
        state
            .ecs_mut()
            .add_resource(SpawnPoint(Vec3::new(16_384.0, 16_384.0, 512.0)));
        state.ecs_mut().add_resource(EventBus::default());

        // Set starting time for the server.
        state.ecs_mut().write_resource::<TimeOfDay>().0 = settings.start_time;

        let mut provider = Provider::new(settings.world_seed, settings.world_folder.clone());
        let save_handle = Some(provider.init_save_loop());

        let this = Self {
            state,
            world_provider: Arc::new(provider),
            save_handle,

            postoffice: PostOffice::bind(addrs.into())?,
            clients: Clients::empty(),

            thread_pool: ThreadPoolBuilder::new()
                .name("veloren-worker".into())
                .build(),
            chunk_tx,
            chunk_rx,
            pending_chunks: HashSet::new(),

            server_info: ServerInfo {
                name: settings.server_name.clone(),
                description: settings.server_description.clone(),
                git_hash: common::util::GIT_HASH.to_string(),
            },
            accounts: AuthProvider::new(),
            server_settings: settings,
        };

        /*let world = this.world.clone();
        std::thread::spawn(move || {
            println!("Saving world");
            world.save().unwrap();
            println!("Done saving!");
        });*/

        Ok(this)
    }

    pub fn with_thread_pool(mut self, thread_pool: ThreadPool) -> Self {
        self.thread_pool = thread_pool;
        self
    }

    /// Get a reference to the server's game state.
    pub fn state(&self) -> &State {
        &self.state
    }
    /// Get a mutable reference to the server's game state.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Get a reference to the server's world.
    pub fn world(&self) -> &World {
        &self.world_provider.world
    }

    /// Build a non-player character.
    pub fn create_npc(
        &mut self,
        pos: comp::Pos,
        name: String,
        body: comp::Body,
    ) -> EcsEntityBuilder {
        self.state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori(Vec3::unit_y()))
            .with(comp::Controller::default())
            .with(body)
            .with(comp::Stats::new(name))
            .with(comp::ActionState::default())
            .with(comp::ForceUpdate)
    }

    /// Build a static object entity
    pub fn create_object(
        &mut self,
        pos: comp::Pos,
        object: comp::object::Body,
    ) -> EcsEntityBuilder {
        self.state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori(Vec3::unit_y()))
            .with(comp::Body::Object(object))
            .with(comp::LightEmitter {
                offset: Vec3::unit_z(),
                ..comp::LightEmitter::default()
            })
            //.with(comp::LightEmitter::default())
            .with(comp::ActionState::default())
            .with(comp::ForceUpdate)
    }

    pub fn create_player_character(
        state: &mut State,
        entity: EcsEntity,
        client: &mut Client,
        name: String,
        body: comp::Body,
        server_settings: &ServerSettings,
    ) {
        let spawn_point = state.ecs().read_resource::<SpawnPoint>().0;

        state.write_component(entity, body);
        state.write_component(entity, comp::Stats::new(name));
        state.write_component(entity, comp::Controller::default());
        state.write_component(entity, comp::Pos(spawn_point));
        state.write_component(entity, comp::Vel(Vec3::zero()));
        state.write_component(entity, comp::Ori(Vec3::unit_y()));
        state.write_component(entity, comp::ActionState::default());
        state.write_component(entity, comp::Inventory::default());
        state.write_component(entity, comp::InventoryUpdate);
        // Make sure physics are accepted.
        state.write_component(entity, comp::ForceUpdate);

        // Give the Admin component to the player if their name exists in admin list
        if server_settings.admins.contains(
            &state
                .ecs()
                .read_storage::<comp::Player>()
                .get(entity)
                .unwrap()
                .alias,
        ) {
            state.write_component(entity, comp::Admin);
        }
        // Tell the client its request was successful.
        client.allow_state(ClientState::Character);
    }

    /// Handle events coming through via the event bus
    fn handle_events(&mut self) {
        let terrain = self.state.ecs().read_resource::<TerrainMap>();
        let mut block_change = self.state.ecs().write_resource::<BlockChange>();
        let mut stats = self.state.ecs().write_storage::<comp::Stats>();

        for event in self.state.ecs().read_resource::<EventBus>().recv_all() {
            match event {
                GameEvent::LandOnGround { entity, vel } => {
                    if let Some(stats) = stats.get_mut(entity) {
                        let falldmg = (vel.z / 1.5 + 10.0) as i32;
                        if falldmg < 0 {
                            stats.health.change_by(falldmg, comp::HealthSource::World);
                        }
                    }
                }
                GameEvent::Explosion { pos, radius } => {
                    const RAYS: usize = 500;

                    for _ in 0..RAYS {
                        let dir = Vec3::new(
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                        )
                        .normalized();

                        let _ = terrain
                            .ray(pos, pos + dir * radius)
                            .until(|_| rand::random::<f32>() < 0.05)
                            .for_each(|pos| block_change.set(pos, Block::empty()))
                            .cast();
                    }
                }
            }
        }
    }

    /// Execute a single server tick, handle input and update the game state by the given duration.
    pub fn tick(&mut self, _input: Input, dt: Duration) -> Result<Vec<Event>, Error> {
        // This tick function is the centre of the Veloren universe. Most server-side things are
        // managed from here, and as such it's important that it stays organised. Please consult
        // the core developers before making significant changes to this code. Here is the
        // approximate order of things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the game
        // 2) Go through any events (timer-driven or otherwise) that need handling and apply them
        //    to the state of the game
        // 3) Go through all incoming client network communications, apply them to the game state
        // 4) Perform a single LocalState tick (i.e: update the world and entities in the world)
        // 5) Go through the terrain update queue and apply all changes to the terrain
        // 6) Send relevant state updates to all clients
        // 7) Finish the tick, passing control of the main thread back to the frontend

        // 1) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // If networking has problems, handle them.
        if let Some(err) = self.postoffice.error() {
            return Err(err.into());
        }

        // 2)

        // 3) Handle inputs from clients
        frontend_events.append(&mut self.handle_new_connections()?);
        frontend_events.append(&mut self.handle_new_messages()?);

        // Handle game events
        self.handle_events();

        // 4) Tick the client's LocalState.
        self.state.tick(dt);

        {
            let mut ecs = self.state.ecs_mut();
            let mut dc = ecs.write_resource::<DirtiedChunks>();
            let map = ecs.read_resource::<TerrainMap>();
            let dirtied = dc.drain();
            for i in dirtied {
                self.world_provider.request_save_chunk(map.get_key(i).unwrap().clone(), i);
            }
        }
        /*self.world_provider.save_chunks(
            self.state.ecs().read_resource::<TerrainMap>().deref(),
            dirtied,
        );*/

        // Tick the world
        self.world().tick(dt);

        // 5) Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // Also, send the chunk data to anybody that is close by.
        if let Ok((key, (chunk, supplement))) = self.chunk_rx.try_recv() {
            // Send the chunk to all nearby players.
            for (entity, view_distance, pos) in (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Player>(),
                &self.state.ecs().read_storage::<comp::Pos>(),
            )
                .join()
                .filter_map(|(entity, player, pos)| {
                    player.view_distance.map(|vd| (entity, vd, pos))
                })
            {
                let chunk_pos = self.state.terrain().pos_key(pos.0.map(|e| e as i32));
                let adjusted_dist_sqr = (Vec2::from(chunk_pos) - Vec2::from(key))
                    .map(|e: i32| (e.abs() as u32).checked_sub(2).unwrap_or(0))
                    .magnitude_squared();

                if adjusted_dist_sqr <= view_distance.pow(2) {
                    self.clients.notify(
                        entity,
                        ServerMsg::TerrainChunkUpdate {
                            key,
                            chunk: Box::new(chunk.clone()),
                        },
                    );
                }
            }

            self.state.insert_chunk(key, chunk);
            self.pending_chunks.remove(&key);

            // Handle chunk supplement
            for npc in supplement.npcs {
                let (mut stats, mut body) = if rand::random() {
                    let stats = comp::Stats::new("Humanoid".to_string());
                    let body = comp::Body::Humanoid(comp::humanoid::Body::random());
                    (stats, body)
                } else {
                    let stats = comp::Stats::new("Wolf".to_string());
                    let body = comp::Body::QuadrupedMedium(comp::quadruped_medium::Body::random());
                    (stats, body)
                };
                let mut scale = 1.0;

                if npc.boss {
                    if rand::random::<f32>() < 0.8 {
                        stats = comp::Stats::new("Humanoid".to_string());
                        body = comp::Body::Humanoid(comp::humanoid::Body::random());
                    }
                    stats = stats.with_max_health(500 + rand::random::<u32>() % 400);
                    scale = 2.5 + rand::random::<f32>();
                }

                self.state
                    .ecs_mut()
                    .create_entity_synced()
                    .with(comp::Pos(npc.pos))
                    .with(comp::Vel(Vec3::zero()))
                    .with(comp::Ori(Vec3::unit_y()))
                    .with(comp::Controller::default())
                    .with(body)
                    .with(stats)
                    .with(comp::ActionState::default())
                    .with(comp::Agent::enemy())
                    .with(comp::Scale(scale))
                    .build();
            }
        }

        fn chunk_in_vd(
            player_pos: Vec3<f32>,
            chunk_pos: Vec2<i32>,
            terrain: &TerrainMap,
            vd: u32,
        ) -> bool {
            let player_chunk_pos = terrain.pos_key(player_pos.map(|e| e as i32));

            let adjusted_dist_sqr = Vec2::from(player_chunk_pos - chunk_pos)
                .map(|e: i32| (e.abs() as u32).checked_sub(2).unwrap_or(0))
                .magnitude_squared();

            adjusted_dist_sqr <= vd.pow(2)
        }

        // Remove chunks that are too far from players.
        let mut chunks_to_remove = Vec::new();
        self.state.terrain().iter().for_each(|(chunk_key, _)| {
            let mut should_drop = true;

            // For each player with a position, calculate the distance.
            for (player, pos) in (
                &self.state.ecs().read_storage::<comp::Player>(),
                &self.state.ecs().read_storage::<comp::Pos>(),
            )
                .join()
            {
                if player
                    .view_distance
                    .map(|vd| chunk_in_vd(pos.0, chunk_key, &self.state.terrain(), vd))
                    .unwrap_or(false)
                {
                    should_drop = false;
                    break;
                }
            }

            if should_drop {
                chunks_to_remove.push(chunk_key);
            }
        });
        for key in chunks_to_remove {
            self.state.remove_chunk(key);
        }

        // 6) Synchronise clients with the new state of the world.
        self.sync_clients();

        // Sync changed chunks
        'chunk: for chunk_key in &self.state.terrain_changes().modified_chunks {
            let terrain = self.state.terrain();

            for (entity, player, pos) in (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Player>(),
                &self.state.ecs().read_storage::<comp::Pos>(),
            )
                .join()
            {
                if player
                    .view_distance
                    .map(|vd| chunk_in_vd(pos.0, *chunk_key, &terrain, vd))
                    .unwrap_or(false)
                {
                    self.clients.notify(
                        entity,
                        ServerMsg::TerrainChunkUpdate {
                            key: *chunk_key,
                            chunk: Box::new(match self.state.terrain().get_key(*chunk_key) {
                                Some(chunk) => chunk.clone(),
                                None => break 'chunk,
                            }),
                        },
                    );
                }
            }
        }

        // Sync changed blocks
        let msg =
            ServerMsg::TerrainBlockUpdates(self.state.terrain_changes().modified_blocks.clone());
        for (entity, player) in (
            &self.state.ecs().entities(),
            &self.state.ecs().read_storage::<comp::Player>(),
        )
            .join()
        {
            if player.view_distance.is_some() {
                self.clients.notify(entity, msg.clone());
            }
        }

        // Remove NPCs that are outside the view distances of all players
        let to_delete = {
            let terrain = self.state.terrain();
            (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Pos>(),
                &self.state.ecs().read_storage::<comp::Agent>(),
            )
                .join()
                .filter(|(_, pos, _)| terrain.get(pos.0.map(|e| e.floor() as i32)).is_err())
                .map(|(entity, _, _)| entity)
                .collect::<Vec<_>>()
        };
        for entity in to_delete {
            let _ = self.state.ecs_mut().delete_entity(entity);
        }

        // 7) Finish the tick, pass control back to the frontend.

        // Cleanup
        let ecs = self.state.ecs_mut();
        for entity in ecs.entities().join() {
            ecs.write_storage::<comp::Dying>().remove(entity);
            ecs.write_storage::<comp::Respawning>().remove(entity);
        }

        Ok(frontend_events)
    }

    /// Clean up the server after a tick.
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handle new client connections.
    fn handle_new_connections(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        for postbox in self.postoffice.new_postboxes() {
            let entity = self.state.ecs_mut().create_entity_synced().build();
            let mut client = Client {
                client_state: ClientState::Connected,
                postbox,
                last_ping: self.state.get_time(),
            };

            if self.server_settings.max_players <= self.clients.len() {
                client.notify(ServerMsg::Error(ServerError::TooManyPlayers));
            } else {
                // Return the state of the current world (all of the components that Sphynx tracks).
                client.notify(ServerMsg::InitialSync {
                    ecs_state: self.state.ecs().gen_state_package(),
                    entity_uid: self.state.ecs().uid_from_entity(entity).unwrap().into(), // Can't fail.
                    server_info: self.server_info.clone(),
                });

                frontend_events.push(Event::ClientConnected { entity });
            }

            self.clients.add(entity, client);
        }

        Ok(frontend_events)
    }

    /// Handle new client messages.
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        let accounts = &mut self.accounts;
        let server_settings = &self.server_settings;

        let state = &mut self.state;
        let mut new_chat_msgs = Vec::new();
        let mut disconnected_clients = Vec::new();
        let mut requested_chunks = Vec::new();
        let mut modified_blocks = Vec::new();
        let mut dropped_items = Vec::new();

        self.clients.remove_if(|entity, client| {
            let mut disconnect = false;
            let new_msgs = client.postbox.new_messages();

            // Update client ping.
            if new_msgs.len() > 0 {
                client.last_ping = state.get_time();

                // Process incoming messages.
                for msg in new_msgs {
                    match msg {
                        ClientMsg::RequestState(requested_state) => match requested_state {
                            ClientState::Connected => disconnect = true, // Default state
                            ClientState::Registered => match client.client_state {
                                // Use ClientMsg::Register instead.
                                ClientState::Connected => {
                                    client.error_state(RequestStateError::WrongMessage)
                                }
                                ClientState::Registered => {
                                    client.error_state(RequestStateError::Already)
                                }
                                ClientState::Spectator
                                | ClientState::Character
                                | ClientState::Dead => client.allow_state(ClientState::Registered),
                                ClientState::Pending => {}
                            },
                            ClientState::Spectator => match requested_state {
                                // Become Registered first.
                                ClientState::Connected => {
                                    client.error_state(RequestStateError::Impossible)
                                }
                                ClientState::Spectator => {
                                    client.error_state(RequestStateError::Already)
                                }
                                ClientState::Registered
                                | ClientState::Character
                                | ClientState::Dead => client.allow_state(ClientState::Spectator),
                                ClientState::Pending => {}
                            },
                            // Use ClientMsg::Character instead.
                            ClientState::Character => {
                                client.error_state(RequestStateError::WrongMessage)
                            }
                            ClientState::Dead => client.error_state(RequestStateError::Impossible),
                            ClientState::Pending => {}
                        },
                        // Valid player
                        ClientMsg::Register { player, password } if player.is_valid() => {
                            if !accounts.query(player.alias.clone(), password) {
                                client.error_state(RequestStateError::Denied);
                                break;
                            }
                            match client.client_state {
                                ClientState::Connected => {
                                    Self::initialize_player(state, entity, client, player);
                                }
                                // Use RequestState instead (No need to send `player` again).
                                _ => client.error_state(RequestStateError::Impossible),
                            }
                            //client.allow_state(ClientState::Registered);
                        }
                        // Invalid player
                        ClientMsg::Register { .. } => {
                            client.error_state(RequestStateError::Impossible)
                        }
                        ClientMsg::SetViewDistance(view_distance) => match client.client_state {
                            ClientState::Character { .. } => {
                                state
                                    .ecs_mut()
                                    .write_storage::<comp::Player>()
                                    .get_mut(entity)
                                    .map(|player| player.view_distance = Some(view_distance));
                            }
                            _ => {}
                        },
                        ClientMsg::SwapInventorySlots(a, b) => {
                            state
                                .ecs()
                                .write_storage::<comp::Inventory>()
                                .get_mut(entity)
                                .map(|inv| inv.swap_slots(a, b));
                            state.write_component(entity, comp::InventoryUpdate);
                        }
                        ClientMsg::DropInventorySlot(x) => {
                            let item = state
                                .ecs()
                                .write_storage::<comp::Inventory>()
                                .get_mut(entity)
                                .and_then(|inv| inv.remove(x));

                            state.write_component(entity, comp::InventoryUpdate);

                            if let (Some(item), Some(pos)) =
                                (item, state.ecs().read_storage::<comp::Pos>().get(entity))
                            {
                                dropped_items.push((
                                    *pos,
                                    state
                                        .ecs()
                                        .read_storage::<comp::Ori>()
                                        .get(entity)
                                        .copied()
                                        .unwrap_or(comp::Ori(Vec3::unit_y())),
                                    item,
                                ));
                            }
                        }
                        ClientMsg::PickUp(uid) => {
                            let item_entity = state.ecs_mut().entity_from_uid(uid);

                            let ecs = state.ecs_mut();

                            let item_entity = if let (Some((item, item_entity)), Some(inv)) = (
                                item_entity.and_then(|item_entity| {
                                    ecs.write_storage::<comp::Item>()
                                        .get_mut(item_entity)
                                        .map(|item| (*item, item_entity))
                                }),
                                ecs.write_storage::<comp::Inventory>().get_mut(entity),
                            ) {
                                if inv.insert(item).is_none() {
                                    Some(item_entity)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            if let Some(item_entity) = item_entity {
                                let _ = ecs.delete_entity_synced(item_entity);
                            }

                            state.write_component(entity, comp::InventoryUpdate);
                        }
                        ClientMsg::Character { name, body } => match client.client_state {
                            // Become Registered first.
                            ClientState::Connected => {
                                client.error_state(RequestStateError::Impossible)
                            }
                            ClientState::Registered
                            | ClientState::Spectator
                            | ClientState::Dead => {
                                Self::create_player_character(
                                    state,
                                    entity,
                                    client,
                                    name,
                                    body,
                                    &server_settings,
                                );
                                if let Some(player) =
                                    state.ecs().read_storage::<comp::Player>().get(entity)
                                {
                                    new_chat_msgs.push((
                                        None,
                                        ServerMsg::broadcast(format!(
                                            "[{}] is now online.",
                                            &player.alias
                                        )),
                                    ));
                                }
                            }
                            ClientState::Character => {
                                client.error_state(RequestStateError::Already)
                            }
                            ClientState::Pending => {}
                        },
                        ClientMsg::Controller(controller) => match client.client_state {
                            ClientState::Connected
                            | ClientState::Registered
                            | ClientState::Spectator => {
                                client.error_state(RequestStateError::Impossible)
                            }
                            ClientState::Dead | ClientState::Character => {
                                state.write_component(entity, controller);
                            }
                            ClientState::Pending => {}
                        },
                        ClientMsg::ChatMsg { chat_type, message } => match client.client_state {
                            ClientState::Connected => {
                                client.error_state(RequestStateError::Impossible)
                            }
                            ClientState::Registered
                            | ClientState::Spectator
                            | ClientState::Dead
                            | ClientState::Character => new_chat_msgs
                                .push((Some(entity), ServerMsg::ChatMsg { chat_type, message })),
                            ClientState::Pending => {}
                        },
                        ClientMsg::PlayerPhysics { pos, vel, ori } => match client.client_state {
                            ClientState::Character => {
                                state.write_component(entity, pos);
                                state.write_component(entity, vel);
                                state.write_component(entity, ori);
                            }
                            // Only characters can send positions.
                            _ => client.error_state(RequestStateError::Impossible),
                        },
                        ClientMsg::BreakBlock(pos) => {
                            if state
                                .ecs_mut()
                                .read_storage::<comp::CanBuild>()
                                .get(entity)
                                .is_some()
                            {
                                modified_blocks.push((pos, Block::empty()));
                            }
                        }
                        ClientMsg::PlaceBlock(pos, block) => {
                            if state
                                .ecs_mut()
                                .read_storage::<comp::CanBuild>()
                                .get(entity)
                                .is_some()
                            {
                                modified_blocks.push((pos, block));
                            }
                        }
                        ClientMsg::TerrainChunkRequest { key } => match client.client_state {
                            ClientState::Connected
                            | ClientState::Registered
                            | ClientState::Dead => {
                                client.error_state(RequestStateError::Impossible);
                            }
                            ClientState::Spectator | ClientState::Character => {
                                match state.terrain().get_key(key) {
                                    Some(chunk) => {
                                        client.postbox.send_message(ServerMsg::TerrainChunkUpdate {
                                            key,
                                            chunk: Box::new(chunk.clone()),
                                        })
                                    }
                                    None => requested_chunks.push(key),
                                }
                            }
                            ClientState::Pending => {}
                        },
                        // Always possible.
                        ClientMsg::Ping => client.postbox.send_message(ServerMsg::Pong),
                        ClientMsg::Pong => {}
                        ClientMsg::Disconnect => {
                            disconnect = true;
                        }
                    }
                }
            } else if state.get_time() - client.last_ping > CLIENT_TIMEOUT || // Timeout
                client.postbox.error().is_some()
            // Postbox error
            {
                disconnect = true;
            } else if state.get_time() - client.last_ping > CLIENT_TIMEOUT * 0.5 {
                // Try pinging the client if the timeout is nearing.
                client.postbox.send_message(ServerMsg::Ping);
            }

            if disconnect {
                if let Some(player) = state.ecs().read_storage::<comp::Player>().get(entity) {
                    new_chat_msgs.push((
                        None,
                        ServerMsg::broadcast(format!("{} went offline.", &player.alias)),
                    ));
                }
                disconnected_clients.push(entity);
                client.postbox.send_message(ServerMsg::Disconnect);
                true
            } else {
                false
            }
        });

        // Handle new chat messages.
        for (entity, msg) in new_chat_msgs {
            match msg {
                ServerMsg::ChatMsg { chat_type, message } => {
                    if let Some(entity) = entity {
                        // Handle chat commands.
                        if message.starts_with("/") && message.len() > 1 {
                            let argv = String::from(&message[1..]);
                            self.process_chat_cmd(entity, argv);
                        } else {
                            let message =
                                match self.state.ecs().read_storage::<comp::Player>().get(entity) {
                                    Some(player) => {
                                        if self.entity_is_admin(entity) {
                                            format!("[ADMIN][{}] {}", &player.alias, message)
                                        } else {
                                            format!("[{}] {}", &player.alias, message)
                                        }
                                    }
                                    None => format!("[<Unknown>] {}", message),
                                };
                            self.clients
                                .notify_registered(ServerMsg::ChatMsg { chat_type, message });
                        }
                    } else {
                        self.clients
                            .notify_registered(ServerMsg::ChatMsg { chat_type, message });
                    }
                }
                _ => {
                    panic!("Invalid message type.");
                }
            }
        }

        // Handle client disconnects.
        for entity in disconnected_clients {
            if let Err(err) = self.state.ecs_mut().delete_entity_synced(entity) {
                debug!("Failed to delete disconnected client: {:?}", err);
            }

            frontend_events.push(Event::ClientDisconnected { entity });
        }

        // Generate requested chunks.
        for key in requested_chunks {
            self.generate_chunk(key);
        }

        for (pos, block) in modified_blocks {
            self.state.set_block(pos, block);
        }

        for (pos, ori, item) in dropped_items {
            let vel = ori.0.normalized() * 5.0
                + Vec3::unit_z() * 10.0
                + Vec3::<f32>::zero().map(|_| rand::thread_rng().gen::<f32>() - 0.5) * 4.0;
            self.create_object(Default::default(), comp::object::Body::Pouch)
                .with(comp::Pos(pos.0 + Vec3::unit_z() * 0.25))
                .with(item)
                .with(comp::Vel(vel))
                .build();
        }

        Ok(frontend_events)
    }

    /// Initialize a new client states with important information.
    fn initialize_player(
        state: &mut State,
        entity: specs::Entity,
        client: &mut Client,
        player: comp::Player,
    ) {
        // Save player metadata (for example the username).
        state.write_component(entity, player);

        // Sync physics of all entities
        for (&uid, &pos, vel, ori, action_state) in (
            &state.ecs().read_storage::<Uid>(),
            &state.ecs().read_storage::<comp::Pos>(), // We assume all these entities have a position
            state.ecs().read_storage::<comp::Vel>().maybe(),
            state.ecs().read_storage::<comp::Ori>().maybe(),
            state.ecs().read_storage::<comp::ActionState>().maybe(),
        )
            .join()
        {
            client.notify(ServerMsg::EntityPos {
                entity: uid.into(),
                pos,
            });
            if let Some(vel) = vel.copied() {
                client.notify(ServerMsg::EntityVel {
                    entity: uid.into(),
                    vel,
                });
            }
            if let Some(ori) = ori.copied() {
                client.notify(ServerMsg::EntityOri {
                    entity: uid.into(),
                    ori,
                });
            }
            if let Some(action_state) = action_state.copied() {
                client.notify(ServerMsg::EntityActionState {
                    entity: uid.into(),
                    action_state,
                });
            }
        }

        // Tell the client its request was successful.
        client.allow_state(ClientState::Registered);
    }

    /// Sync client states with the most up to date information.
    fn sync_clients(&mut self) {
        // Sync 'logical' state using Sphynx.
        self.clients
            .notify_registered(ServerMsg::EcsSync(self.state.ecs_mut().next_sync_package()));

        // TODO: Move this into some new method like `handle_sys_outputs` right after ticking the world
        // Handle deaths.
        let ecs = self.state.ecs_mut();
        let clients = &mut self.clients;
        let todo_kill = (&ecs.entities(), &ecs.read_storage::<comp::Dying>())
            .join()
            .map(|(entity, dying)| {
                // Chat message
                if let Some(player) = ecs.read_storage::<comp::Player>().get(entity) {
                    let msg = if let comp::HealthSource::Attack { by } = dying.cause {
                        ecs.entity_from_uid(by.into()).and_then(|attacker| {
                            ecs.read_storage::<comp::Player>()
                                .get(attacker)
                                .map(|attacker_alias| {
                                    format!(
                                        "{} was killed by {}",
                                        &player.alias, &attacker_alias.alias
                                    )
                                })
                        })
                    } else {
                        None
                    }
                    .unwrap_or(format!("{} died", &player.alias));

                    clients.notify_registered(ServerMsg::kill(msg));
                }

                // Give EXP to the client
                let mut stats = ecs.write_storage::<comp::Stats>();
                if let Some(entity_stats) = stats.get(entity).cloned() {
                    if let comp::HealthSource::Attack { by } = dying.cause {
                        ecs.entity_from_uid(by.into()).map(|attacker| {
                            if let Some(attacker_stats) = stats.get_mut(attacker) {
                                // TODO: Discuss whether we should give EXP by Player Killing or not.
                                attacker_stats.exp.change_by(
                                    entity_stats.health.maximum() as f64 / 10.0
                                        + entity_stats.level.level() as f64 * 10.0,
                                );
                            }
                        });
                    }
                }

                entity
            })
            .collect::<Vec<_>>();

        // Actually kill them
        for entity in todo_kill {
            if let Some(client) = self.clients.get_mut(&entity) {
                let _ = ecs.write_storage().insert(entity, comp::Vel(Vec3::zero()));
                let _ = ecs.write_storage().insert(entity, comp::ForceUpdate);
                client.force_state(ClientState::Dead);
            } else {
                let _ = ecs.delete_entity_synced(entity);
                continue;
            }
        }

        // Handle respawns
        let todo_respawn = (&ecs.entities(), &ecs.read_storage::<comp::Respawning>())
            .join()
            .map(|(entity, _)| entity)
            .collect::<Vec<EcsEntity>>();

        for entity in todo_respawn {
            if let Some(client) = self.clients.get_mut(&entity) {
                client.allow_state(ClientState::Character);
                ecs.write_storage::<comp::Stats>()
                    .get_mut(entity)
                    .map(|stats| stats.revive());
                ecs.write_storage::<comp::Pos>()
                    .get_mut(entity)
                    .map(|pos| pos.0.z += 20.0);
                let _ = ecs.write_storage().insert(entity, comp::ForceUpdate);
            }
        }

        // Sync physics
        for (entity, &uid, &pos, force_update) in (
            &ecs.entities(),
            &ecs.read_storage::<Uid>(),
            &ecs.read_storage::<comp::Pos>(),
            ecs.read_storage::<comp::ForceUpdate>().maybe(),
        )
            .join()
        {
            let clients = &mut self.clients;

            let in_vd = |entity| {
                if let (Some(client_pos), Some(client_vd)) = (
                    ecs.read_storage::<comp::Pos>().get(entity),
                    ecs.read_storage::<comp::Player>()
                        .get(entity)
                        .map(|pl| pl.view_distance)
                        .and_then(|v| v),
                ) {
                    {
                        // Check if the entity is in the client's range
                        (pos.0 - client_pos.0)
                            .map2(TerrainChunkSize::SIZE, |d, sz| {
                                (d.abs() as u32 / sz).checked_sub(2).unwrap_or(0)
                            })
                            .magnitude_squared()
                            < client_vd.pow(2)
                    }
                } else {
                    false
                }
            };

            let mut last_pos = ecs.write_storage::<comp::Last<comp::Pos>>();
            let mut last_vel = ecs.write_storage::<comp::Last<comp::Vel>>();
            let mut last_ori = ecs.write_storage::<comp::Last<comp::Ori>>();
            let mut last_action_state = ecs.write_storage::<comp::Last<comp::ActionState>>();

            if let Some(client_pos) = ecs.read_storage::<comp::Pos>().get(entity) {
                if last_pos
                    .get(entity)
                    .map(|&l| l != *client_pos)
                    .unwrap_or(true)
                {
                    let _ = last_pos.insert(entity, comp::Last(*client_pos));
                    let msg = ServerMsg::EntityPos {
                        entity: uid.into(),
                        pos: *client_pos,
                    };
                    match force_update {
                        Some(_) => clients.notify_ingame_if(msg, in_vd),
                        None => clients.notify_ingame_if_except(entity, msg, in_vd),
                    }
                }
            }

            if let Some(client_vel) = ecs.read_storage::<comp::Vel>().get(entity) {
                if last_vel
                    .get(entity)
                    .map(|&l| l != *client_vel)
                    .unwrap_or(true)
                {
                    let _ = last_vel.insert(entity, comp::Last(*client_vel));
                    let msg = ServerMsg::EntityVel {
                        entity: uid.into(),
                        vel: *client_vel,
                    };
                    match force_update {
                        Some(_) => clients.notify_ingame_if(msg, in_vd),
                        None => clients.notify_ingame_if_except(entity, msg, in_vd),
                    }
                }
            }

            if let Some(client_ori) = ecs.read_storage::<comp::Ori>().get(entity) {
                if last_ori
                    .get(entity)
                    .map(|&l| l != *client_ori)
                    .unwrap_or(true)
                {
                    let _ = last_ori.insert(entity, comp::Last(*client_ori));
                    let msg = ServerMsg::EntityOri {
                        entity: uid.into(),
                        ori: *client_ori,
                    };
                    match force_update {
                        Some(_) => clients.notify_ingame_if(msg, in_vd),
                        None => clients.notify_ingame_if_except(entity, msg, in_vd),
                    }
                }
            }

            if let Some(client_action_state) = ecs.read_storage::<comp::ActionState>().get(entity) {
                if last_action_state
                    .get(entity)
                    .map(|&l| l != *client_action_state)
                    .unwrap_or(true)
                {
                    let _ = last_action_state.insert(entity, comp::Last(*client_action_state));
                    let msg = ServerMsg::EntityActionState {
                        entity: uid.into(),
                        action_state: *client_action_state,
                    };
                    match force_update {
                        Some(_) => clients.notify_ingame_if(msg, in_vd),
                        None => clients.notify_ingame_if_except(entity, msg, in_vd),
                    }
                }
            }
        }

        // Sync inventories
        for (entity, inventory, _) in (
            &self.state.ecs().entities(),
            &self.state.ecs().read_storage::<comp::Inventory>(),
            &self.state.ecs().read_storage::<comp::InventoryUpdate>(),
        )
            .join()
        {
            self.clients
                .notify(entity, ServerMsg::InventoryUpdate(inventory.clone()));
        }

        // Remove all force flags.
        self.state
            .ecs_mut()
            .write_storage::<comp::ForceUpdate>()
            .clear();
        self.state
            .ecs_mut()
            .write_storage::<comp::InventoryUpdate>()
            .clear();
    }

    pub fn generate_chunk(&mut self, key: Vec2<i32>) {
        if self.pending_chunks.insert(key) {
            let chunk_tx = self.chunk_tx.clone();
            let world = self.world_provider.clone();
            self.thread_pool.execute(move || {
                let _ = chunk_tx.send((key, world.get_chunk(key)));
            });
        }
    }

    fn process_chat_cmd(&mut self, entity: EcsEntity, cmd: String) {
        // Separate string into keyword and arguments.
        let sep = cmd.find(' ');
        let (kwd, args) = match sep {
            Some(i) => (cmd[..i].to_string(), cmd[(i + 1)..].to_string()),
            None => (cmd, "".to_string()),
        };

        // Find the command object and run its handler.
        let action_opt = CHAT_COMMANDS.iter().find(|x| x.keyword == kwd);
        match action_opt {
            Some(action) => action.execute(self, entity, args),
            // Unknown command
            None => {
                self.clients.notify(
                    entity,
                    ServerMsg::private(format!(
                        "Unrecognised command: '/{}'\ntype '/help' for a list of available commands",
                        kwd
                    )),
                );
            }
        }
    }

    fn entity_is_admin(&self, entity: EcsEntity) -> bool {
        self.state
            .read_storage::<comp::Admin>()
            .get(entity)
            .is_some()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.clients.notify_registered(ServerMsg::Shutdown);
        self.save_handle.take().unwrap().join();
    }
}
