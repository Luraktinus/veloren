use crate::{
    mesh::Meshable,
    render::{
        Consts, FluidPipeline, Globals, Instances, Light, Mesh, Model, Renderer, SpriteInstance,
        SpritePipeline, TerrainLocals, TerrainPipeline,
    },
};
use client::Client;
use common::{
    assets,
    figure::Segment,
    terrain::{Block, BlockKind, TerrainChunkSize, TerrainMap},
    vol::{ReadVol, SampleVol, VolSize, Vox},
    volumes::vol_map_2d::VolMap2dErr,
};
use crossbeam::channel;
use dot_vox::DotVoxData;
use frustum_query::frustum::Frustum;
use hashbrown::HashMap;
use std::{f32, i32, ops::Mul, time::Duration};
use vek::*;

struct TerrainChunk {
    // GPU data
    opaque_model: Model<TerrainPipeline>,
    fluid_model: Model<FluidPipeline>,
    sprite_instances: HashMap<(BlockKind, usize), Instances<SpriteInstance>>,
    locals: Consts<TerrainLocals>,

    visible: bool,
    z_bounds: (f32, f32),
}

struct ChunkMeshState {
    pos: Vec2<i32>,
    started_tick: u64,
    active_worker: Option<u64>,
}

/// A type produced by mesh worker threads corresponding to the position and mesh of a chunk.
struct MeshWorkerResponse {
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    opaque_mesh: Mesh<TerrainPipeline>,
    fluid_mesh: Mesh<FluidPipeline>,
    sprite_instances: HashMap<(BlockKind, usize), Vec<SpriteInstance>>,
    started_tick: u64,
}

struct SpriteConfig {
    variations: usize,
    wind_sway: f32, // 1.0 is normal
}

fn sprite_config_for(kind: BlockKind) -> Option<SpriteConfig> {
    match kind {
        BlockKind::LargeCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::BarrelCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),

        BlockKind::BlueFlower => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.3,
        }),
        BlockKind::PinkFlower => Some(SpriteConfig {
            variations: 3,
            wind_sway: 0.3,
        }),
        BlockKind::RedFlower => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.3,
        }),
        BlockKind::WhiteFlower => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.3,
        }),
        BlockKind::YellowFlower => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.3,
        }),
        BlockKind::Sunflower => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.3,
        }),

        BlockKind::LongGrass => Some(SpriteConfig {
            variations: 5,
            wind_sway: 1.0,
        }),
        BlockKind::MediumGrass => Some(SpriteConfig {
            variations: 5,
            wind_sway: 1.0,
        }),
        BlockKind::ShortGrass => Some(SpriteConfig {
            variations: 5,
            wind_sway: 1.0,
        }),

        BlockKind::Apple => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        _ => None,
    }
}

/// Function executed by worker threads dedicated to chunk meshing.
fn mesh_worker(
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    started_tick: u64,
    volume: <TerrainMap as SampleVol<Aabr<i32>>>::Sample,
    range: Aabb<i32>,
) -> MeshWorkerResponse {
    let (opaque_mesh, fluid_mesh) = volume.generate_mesh(range);
    MeshWorkerResponse {
        pos,
        z_bounds,
        opaque_mesh,
        fluid_mesh,
        // Extract sprite locations from volume
        sprite_instances: {
            let mut instances = HashMap::new();

            for x in 0..TerrainChunkSize::SIZE.x as i32 {
                for y in 0..TerrainChunkSize::SIZE.y as i32 {
                    for z in z_bounds.0 as i32..z_bounds.1 as i32 + 1 {
                        let wpos = Vec3::from(
                            pos * Vec2::from(TerrainChunkSize::SIZE).map(|e: u32| e as i32),
                        ) + Vec3::new(x, y, z);

                        let kind = volume.get(wpos).unwrap_or(&Block::empty()).kind();

                        if let Some(cfg) = sprite_config_for(kind) {
                            let seed = wpos.x * 3 + wpos.y * 7 + wpos.z * 13 + wpos.x * wpos.y;

                            let instance = SpriteInstance::new(
                                Mat4::identity()
                                    .rotated_z(f32::consts::PI * 0.5 * (seed % 4) as f32)
                                    .translated_3d(
                                        wpos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0),
                                    ),
                                Rgb::broadcast(1.0),
                                cfg.wind_sway,
                            );

                            instances
                                .entry((kind, seed as usize % cfg.variations))
                                .or_insert_with(|| Vec::new())
                                .push(instance);
                        }
                    }
                }
            }

            instances
        },
        started_tick,
    }
}

pub struct Terrain {
    chunks: HashMap<Vec2<i32>, TerrainChunk>,

    // The mpsc sender and receiver used for talking to meshing worker threads.
    // We keep the sender component for no reason other than to clone it and send it to new workers.
    mesh_send_tmp: channel::Sender<MeshWorkerResponse>,
    mesh_recv: channel::Receiver<MeshWorkerResponse>,
    mesh_todo: HashMap<Vec2<i32>, ChunkMeshState>,

    // GPU data
    sprite_models: HashMap<(BlockKind, usize), Model<SpritePipeline>>,
}

impl Terrain {
    pub fn new(renderer: &mut Renderer) -> Self {
        // Create a new mpsc (Multiple Produced, Single Consumer) pair for communicating with
        // worker threads that are meshing chunks.
        let (send, recv) = channel::unbounded();

        let mut make_model = |s| {
            renderer
                .create_model(
                    &Meshable::<SpritePipeline, SpritePipeline>::generate_mesh(
                        &Segment::from(assets::load_expect::<DotVoxData>(s).as_ref()),
                        Vec3::new(-6.0, -6.0, 0.0),
                    )
                    .0,
                )
                .unwrap()
        };

        Self {
            chunks: HashMap::default(),
            mesh_send_tmp: send,
            mesh_recv: recv,
            mesh_todo: HashMap::default(),
            sprite_models: vec![
                // Cacti
                (
                    (BlockKind::LargeCactus, 0),
                    make_model("voxygen.voxel.sprite.cacti.large_cactus"),
                ),
                (
                    (BlockKind::BarrelCactus, 0),
                    make_model("voxygen.voxel.sprite.cacti.barrel_cactus"),
                ),
                // Fruit
                (
                    (BlockKind::Apple, 0),
                    make_model("voxygen.voxel.sprite.fruit.apple"),
                ),
                // Flowers
                (
                    (BlockKind::BlueFlower, 0),
                    make_model("voxygen.voxel.sprite.flowers.flower_blue_1"),
                ),
                (
                    (BlockKind::BlueFlower, 1),
                    make_model("voxygen.voxel.sprite.flowers.flower_blue_2"),
                ),
                (
                    (BlockKind::PinkFlower, 0),
                    make_model("voxygen.voxel.sprite.flowers.flower_pink_1"),
                ),
                (
                    (BlockKind::PinkFlower, 1),
                    make_model("voxygen.voxel.sprite.flowers.flower_pink_2"),
                ),
                (
                    (BlockKind::PinkFlower, 2),
                    make_model("voxygen.voxel.sprite.flowers.flower_pink_3"),
                ),
                (
                    (BlockKind::PurpleFlower, 0),
                    make_model("voxygen.voxel.sprite.flowers.flower_purple_1"),
                ),
                (
                    (BlockKind::RedFlower, 0),
                    make_model("voxygen.voxel.sprite.flowers.flower_red_1"),
                ),
                (
                    (BlockKind::WhiteFlower, 0),
                    make_model("voxygen.voxel.sprite.flowers.flower_white_1"),
                ),
                (
                    (BlockKind::YellowFlower, 0),
                    make_model("voxygen.voxel.sprite.flowers.flower_purple_1"),
                ),
                (
                    (BlockKind::Sunflower, 0),
                    make_model("voxygen.voxel.sprite.flowers.sunflower_1"),
                ),
                (
                    (BlockKind::Sunflower, 1),
                    make_model("voxygen.voxel.sprite.flowers.sunflower_2"),
                ),
                // Grass
                (
                    (BlockKind::LongGrass, 0),
                    make_model("voxygen.voxel.sprite.grass.grass_long_1"),
                ),
                (
                    (BlockKind::LongGrass, 1),
                    make_model("voxygen.voxel.sprite.grass.grass_long_2"),
                ),
                (
                    (BlockKind::LongGrass, 2),
                    make_model("voxygen.voxel.sprite.grass.grass_long_3"),
                ),
                (
                    (BlockKind::LongGrass, 3),
                    make_model("voxygen.voxel.sprite.grass.grass_long_4"),
                ),
                (
                    (BlockKind::LongGrass, 4),
                    make_model("voxygen.voxel.sprite.grass.grass_long_5"),
                ),
                (
                    (BlockKind::MediumGrass, 0),
                    make_model("voxygen.voxel.sprite.grass.grass_med_1"),
                ),
                (
                    (BlockKind::MediumGrass, 1),
                    make_model("voxygen.voxel.sprite.grass.grass_med_2"),
                ),
                (
                    (BlockKind::MediumGrass, 2),
                    make_model("voxygen.voxel.sprite.grass.grass_med_3"),
                ),
                (
                    (BlockKind::MediumGrass, 3),
                    make_model("voxygen.voxel.sprite.grass.grass_med_4"),
                ),
                (
                    (BlockKind::MediumGrass, 4),
                    make_model("voxygen.voxel.sprite.grass.grass_med_5"),
                ),
                (
                    (BlockKind::ShortGrass, 0),
                    make_model("voxygen.voxel.sprite.grass.grass_short_1"),
                ),
                (
                    (BlockKind::ShortGrass, 1),
                    make_model("voxygen.voxel.sprite.grass.grass_short_2"),
                ),
                (
                    (BlockKind::ShortGrass, 2),
                    make_model("voxygen.voxel.sprite.grass.grass_short_3"),
                ),
                (
                    (BlockKind::ShortGrass, 3),
                    make_model("voxygen.voxel.sprite.grass.grass_short_3"),
                ),
                (
                    (BlockKind::ShortGrass, 4),
                    make_model("voxygen.voxel.sprite.grass.grass_short_5"),
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    /// Maintain terrain data. To be called once per tick.
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        client: &Client,
        focus_pos: Vec3<f32>,
        loaded_distance: f32,
        view_mat: Mat4<f32>,
        proj_mat: Mat4<f32>,
    ) {
        let current_tick = client.get_tick();

        // Add any recently created or changed chunks to the list of chunks to be meshed.
        for (modified, pos) in client
            .state()
            .terrain_changes()
            .modified_chunks
            .iter()
            .map(|c| (true, c))
            .chain(
                client
                    .state()
                    .terrain_changes()
                    .new_chunks
                    .iter()
                    .map(|c| (false, c)),
            )
        {
            // TODO: ANOTHER PROBLEM HERE!
            // What happens if the block on the edge of a chunk gets modified? We need to spawn
            // a mesh worker to remesh its neighbour(s) too since their ambient occlusion and face
            // elision information changes too!
            for i in -1..2 {
                for j in -1..2 {
                    let pos = pos + Vec2::new(i, j);

                    if !self.chunks.contains_key(&pos) || modified {
                        let mut neighbours = true;
                        for i in -1..2 {
                            for j in -1..2 {
                                neighbours &= client
                                    .state()
                                    .terrain()
                                    .get_key(pos + Vec2::new(i, j))
                                    .is_some();
                            }
                        }

                        if neighbours {
                            self.mesh_todo.insert(
                                pos,
                                ChunkMeshState {
                                    pos,
                                    started_tick: current_tick,
                                    active_worker: None,
                                },
                            );
                        }
                    }
                }
            }
        }

        // Add the chunks belonging to recently changed blocks to the list of chunks to be meshed
        for pos in client
            .state()
            .terrain_changes()
            .modified_blocks
            .iter()
            .map(|(p, _)| *p)
        {
            let chunk_pos = client.state().terrain().pos_key(pos);

            self.mesh_todo.insert(
                chunk_pos,
                ChunkMeshState {
                    pos: chunk_pos,
                    started_tick: current_tick,
                    active_worker: None,
                },
            );

            // Handle chunks on chunk borders
            for x in -1..2 {
                for y in -1..2 {
                    let neighbour_pos = pos + Vec3::new(x, y, 0);
                    let neighbour_chunk_pos = client.state().terrain().pos_key(neighbour_pos);

                    if neighbour_chunk_pos != chunk_pos {
                        self.mesh_todo.insert(
                            neighbour_chunk_pos,
                            ChunkMeshState {
                                pos: neighbour_chunk_pos,
                                started_tick: current_tick,
                                active_worker: None,
                            },
                        );
                    }
                }
            }
        }

        // Remove any models for chunks that have been recently removed.
        for pos in &client.state().terrain_changes().removed_chunks {
            self.chunks.remove(pos);
            self.mesh_todo.remove(pos);
        }

        for todo in self
            .mesh_todo
            .values_mut()
            .filter(|todo| {
                todo.active_worker
                    .map(|worker_tick| worker_tick < todo.started_tick)
                    .unwrap_or(true)
            })
            .min_by_key(|todo| todo.active_worker.unwrap_or(todo.started_tick))
        {
            if client.thread_pool().queued_jobs() > 0 {
                break;
            }

            // Find the area of the terrain we want. Because meshing needs to compute things like
            // ambient occlusion and edge elision, we also need the borders of the chunk's
            // neighbours too (hence the `- 1` and `+ 1`).
            let aabr = Aabr {
                min: todo
                    .pos
                    .map2(TerrainMap::chunk_size(), |e, sz| e * sz as i32 - 1),
                max: todo
                    .pos
                    .map2(TerrainMap::chunk_size(), |e, sz| (e + 1) * sz as i32 + 1),
            };

            // Copy out the chunk data we need to perform the meshing. We do this by taking a
            // sample of the terrain that includes both the chunk we want and its neighbours.
            let volume = match client.state().terrain().sample(aabr) {
                Ok(sample) => sample,
                // Either this chunk or its neighbours doesn't yet exist, so we keep it in the
                // queue to be processed at a later date when we have its neighbours.
                Err(VolMap2dErr::NoSuchChunk) => return,
                _ => panic!("Unhandled edge case"),
            };

            // The region to actually mesh
            let min_z = volume
                .iter()
                .fold(i32::MAX, |min, (_, chunk)| chunk.get_min_z().min(min));
            let max_z = volume
                .iter()
                .fold(i32::MIN, |max, (_, chunk)| chunk.get_max_z().max(max));

            let aabb = Aabb {
                min: Vec3::from(aabr.min) + Vec3::unit_z() * (min_z - 1),
                max: Vec3::from(aabr.max) + Vec3::unit_z() * (max_z + 1),
            };

            // Clone various things so that they can be moved into the thread.
            let send = self.mesh_send_tmp.clone();
            let pos = todo.pos;

            // Queue the worker thread.
            let started_tick = todo.started_tick;
            client.thread_pool().execute(move || {
                let _ = send.send(mesh_worker(
                    pos,
                    (min_z as f32, max_z as f32),
                    started_tick,
                    volume,
                    aabb,
                ));
            });
            todo.active_worker = Some(todo.started_tick);
        }

        // Receive a chunk mesh from a worker thread and upload it to the GPU, then store it.
        // Only pull out one chunk per frame to avoid an unacceptable amount of blocking lag due
        // to the GPU upload. That still gives us a 60 chunks / second budget to play with.
        if let Ok(response) = self.mesh_recv.recv_timeout(Duration::new(0, 0)) {
            match self.mesh_todo.get(&response.pos) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course).
                Some(todo) if response.started_tick <= todo.started_tick => {
                    self.chunks.insert(
                        response.pos,
                        TerrainChunk {
                            opaque_model: renderer
                                .create_model(&response.opaque_mesh)
                                .expect("Failed to upload chunk mesh to the GPU!"),
                            fluid_model: renderer
                                .create_model(&response.fluid_mesh)
                                .expect("Failed to upload chunk mesh to the GPU!"),
                            sprite_instances: response
                                .sprite_instances
                                .into_iter()
                                .map(|(kind, instances)| {
                                    (
                                        kind,
                                        renderer.create_instances(&instances).expect(
                                            "Failed to upload chunk sprite instances to the GPU!",
                                        ),
                                    )
                                })
                                .collect(),
                            locals: renderer
                                .create_consts(&[TerrainLocals {
                                    model_offs: Vec3::from(
                                        response.pos.map2(TerrainMap::chunk_size(), |e, sz| {
                                            e as f32 * sz as f32
                                        }),
                                    )
                                    .into_array(),
                                }])
                                .expect("Failed to upload chunk locals to the GPU!"),
                            visible: false,
                            z_bounds: response.z_bounds,
                        },
                    );

                    if response.started_tick == todo.started_tick {
                        self.mesh_todo.remove(&response.pos);
                    }
                }
                // Chunk must have been removed, or it was spawned on an old tick. Drop the mesh
                // since it's either out of date or no longer needed.
                _ => {}
            }
        }

        // Construct view frustum
        let frustum = Frustum::from_modelview_and_projection(
            &view_mat.into_col_array(),
            &proj_mat.into_col_array(),
        );

        // Update chunk visibility
        let chunk_sz = TerrainChunkSize::SIZE.x as f32;
        for (pos, chunk) in &mut self.chunks {
            let chunk_pos = pos.map(|e| e as f32 * chunk_sz);

            // Limit focus_pos to chunk bounds and ensure the chunk is within the fog boundary
            let nearest_in_chunk = Vec2::from(focus_pos).clamped(chunk_pos, chunk_pos + chunk_sz);
            let in_range = Vec2::<f32>::from(focus_pos).distance_squared(nearest_in_chunk)
                < loaded_distance.powf(2.0);

            // Ensure the chunk is within the view frustrum
            let chunk_mid = Vec3::new(
                chunk_pos.x + chunk_sz / 2.0,
                chunk_pos.y + chunk_sz / 2.0,
                (chunk.z_bounds.0 + chunk.z_bounds.1) * 0.5,
            );
            let chunk_radius = ((chunk.z_bounds.1 - chunk.z_bounds.0) / 2.0)
                .max(chunk_sz / 2.0)
                .powf(2.0)
                .mul(2.0)
                .sqrt();
            let in_frustum = frustum.sphere_intersecting(
                &chunk_mid.x,
                &chunk_mid.y,
                &chunk_mid.z,
                &chunk_radius,
            );

            chunk.visible = in_range && in_frustum;
        }
    }

    pub fn render(
        &self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        focus_pos: Vec3<f32>,
    ) {
        // Opaque
        for (pos, chunk) in &self.chunks {
            if chunk.visible {
                renderer.render_terrain_chunk(&chunk.opaque_model, globals, &chunk.locals, lights);

                const SPRITE_RENDER_DISTANCE: f32 = 128.0;

                let chunk_center = pos.map2(Vec2::from(TerrainChunkSize::SIZE), |e, sz: u32| {
                    (e as f32 + 0.5) * sz as f32
                });
                if Vec2::from(focus_pos).distance_squared(chunk_center)
                    < SPRITE_RENDER_DISTANCE * SPRITE_RENDER_DISTANCE
                {
                    for (kind, instances) in &chunk.sprite_instances {
                        renderer.render_sprites(
                            &self.sprite_models[&kind],
                            globals,
                            &instances,
                            lights,
                        );
                    }
                }
            }
        }

        // Translucent
        for (_, chunk) in &self.chunks {
            if chunk.visible {
                renderer.render_fluid_chunk(&chunk.fluid_model, globals, &chunk.locals, lights);
            }
        }
    }
}
