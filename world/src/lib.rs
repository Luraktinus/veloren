#![feature(euclidean_division, bind_by_move_pattern_guards, option_flattening)]

mod all;
mod block;
mod column;
pub mod config;
pub mod sim;
pub mod util;

// Reexports
pub use crate::config::CONFIG;

use crate::{
    block::BlockGen,
    column::{ColumnGen, ColumnSample},
    util::{Sampler, SamplerMut},
};
use common::{
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize, TerrainMap},
    vol::{ReadVol, VolSize, Vox, WriteVol},
};
use rand::Rng;
use rand_chacha::ChaChaRng;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use vek::*;

#[derive(Debug)]
pub enum Error {
    Other(String),
}

pub struct World {
    sim: sim::WorldSim,
    target: PathBuf,
}

fn qser<T: serde::Serialize>(t: PathBuf, obj: &T) -> std::io::Result<()> {
    let out = File::create(t)?;
    bincode::serialize_into(out, obj).unwrap();
    Ok(())
}

fn qdeser<T: serde::de::DeserializeOwned>(t: PathBuf) -> std::io::Result<T> {
    let r = File::open(t)?;
    let val = bincode::deserialize_from(r).unwrap();
    Ok(val)
}

impl World {
    pub fn new(seed: u32, target: PathBuf) -> Self {
        if target.is_dir() {
            return World::load(target.clone()).unwrap_or_else(|_| {
                //println!("Failed to open {:?}/, moving to {:?}.old/", target, target);
                //std::fs::rename(target.clone(), target.clone().with_extension("old"))
                //    .unwrap_or_else(|_| println!("Ok, something strange is happening here..."));
                World::generate(seed, target)
            });
        }
        World::generate(seed, target)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let t = |val: &str| self.target.join(val);
        qser(t("chunks"), &self.sim.chunks)?;
        qser(t("locations"), &self.sim.locations)?;
        qser(t("seed"), &self.sim.seed)?;

        Ok(())
    }

    pub fn chunk_name(v: Vec2<i32>) -> String {
        format!("{}_{}", v.x, v.y)
    }

    pub fn chunk_path(&self, v: Vec2<i32>) -> PathBuf {
        self.target.join(Self::chunk_name(v))
    }

    pub fn save_chunks<T: IntoIterator<Item = Vec2<i32>>>(&self, map: &TerrainMap, chunks: T) {
        for chunk in chunks {
            qser(self.chunk_path(chunk), map.get_key(chunk).unwrap()).unwrap();
        }
    }

    pub fn load(target: PathBuf) -> std::io::Result<Self> {
        let t = |val: &str| target.join(val);
        let chunks = qdeser(t("chunks"))?;
        let locations = qdeser(t("locations"))?;
        let mut seed = qdeser(t("seed"))?;
        let gen_ctx = sim::GenCtx::from_seed(&mut seed);

        Ok(Self {
            sim: sim::WorldSim {
                chunks,
                locations,
                seed,
                gen_ctx,
                rng: sim::get_rng(seed),
            },
            target,
        })
    }

    pub fn generate(seed: u32, target: PathBuf) -> Self {
        std::fs::create_dir_all(target.clone()).unwrap();
        Self {
            sim: sim::WorldSim::generate(seed),
            target,
        }
    }

    pub fn sim(&self) -> &sim::WorldSim {
        &self.sim
    }

    pub fn tick(&self, _dt: Duration) {
        // TODO
    }

    pub fn sample_columns(
        &self,
    ) -> impl Sampler<Index = Vec2<i32>, Sample = Option<ColumnSample>> + '_ {
        ColumnGen::new(self)
    }

    pub fn sample_blocks(&self) -> BlockGen {
        BlockGen::new(self, ColumnGen::new(self))
    }

    pub fn get_chunk(&self, chunk_pos: Vec2<i32>) -> (TerrainChunk, ChunkSupplement) {
        match qdeser(self.chunk_path(chunk_pos)) {
            Ok(chunk) => (chunk, ChunkSupplement::default()),
            Err(_) => self.generate_chunk(chunk_pos),
        }
    }

    pub fn generate_chunk(&self, chunk_pos: Vec2<i32>) -> (TerrainChunk, ChunkSupplement) {
        let air = Block::empty();
        let stone = Block::new(2, Rgb::new(200, 220, 255));
        let water = Block::new(5, Rgb::new(100, 150, 255));

        let chunk_size2d = Vec2::from(TerrainChunkSize::SIZE);
        let (base_z, sim_chunk) = match self
            .sim
            .get_interpolated(
                chunk_pos.map2(chunk_size2d, |e, sz: u32| e * sz as i32 + sz as i32 / 2),
                |chunk| chunk.get_base_z(),
            )
            .and_then(|base_z| self.sim.get(chunk_pos).map(|sim_chunk| (base_z, sim_chunk)))
        {
            Some((base_z, sim_chunk)) => (base_z as i32, sim_chunk),
            None => {
                return (
                    TerrainChunk::new(
                        CONFIG.sea_level as i32,
                        water,
                        air,
                        TerrainChunkMeta::void(),
                    ),
                    ChunkSupplement::default(),
                )
            }
        };

        let meta = TerrainChunkMeta::new(sim_chunk.get_name(&self.sim), sim_chunk.get_biome());
        let mut sampler = self.sample_blocks();

        let chunk_block_pos = Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);

        let mut chunk = TerrainChunk::new(base_z, stone, air, meta);
        for x in 0..TerrainChunkSize::SIZE.x as i32 {
            for y in 0..TerrainChunkSize::SIZE.y as i32 {
                let wpos2d = Vec2::new(x, y)
                    + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);

                let z_cache = match sampler.get_z_cache(wpos2d) {
                    Some(z_cache) => z_cache,
                    None => continue,
                };

                let (min_z, max_z) = z_cache.get_z_limits();

                for z in base_z..min_z as i32 {
                    let _ = chunk.set(Vec3::new(x, y, z), stone);
                }

                for z in min_z as i32..max_z as i32 {
                    let lpos = Vec3::new(x, y, z);
                    let wpos = chunk_block_pos + lpos;

                    if let Some(block) = sampler.get_with_z_cache(wpos, Some(&z_cache)) {
                        let _ = chunk.set(lpos, block);
                    }
                }
            }
        }

        let gen_entity_pos = || {
            let lpos2d = Vec2::from(TerrainChunkSize::SIZE)
                .map(|sz| rand::thread_rng().gen::<u32>().rem_euclid(sz));
            let mut lpos = Vec3::new(lpos2d.x as i32, lpos2d.y as i32, 0);

            while chunk.get(lpos).map(|vox| !vox.is_empty()).unwrap_or(false) {
                lpos.z += 1;
            }

            (chunk_block_pos + lpos).map(|e| e as f32) + 0.5
        };

        const SPAWN_RATE: f32 = 0.1;
        const BOSS_RATE: f32 = 0.03;
        let supplement = ChunkSupplement {
            npcs: if rand::thread_rng().gen::<f32>() < SPAWN_RATE && sim_chunk.chaos < 0.5 {
                vec![NpcInfo {
                    pos: gen_entity_pos(),
                    boss: rand::thread_rng().gen::<f32>() < BOSS_RATE,
                }]
            } else {
                Vec::new()
            },
        };

        (chunk, supplement)
    }
}

pub struct NpcInfo {
    pub pos: Vec3<f32>,
    pub boss: bool,
}

pub struct ChunkSupplement {
    pub npcs: Vec<NpcInfo>,
}

impl Default for ChunkSupplement {
    fn default() -> Self {
        Self { npcs: Vec::new() }
    }
}
