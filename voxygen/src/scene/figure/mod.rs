mod cache;
mod load;

pub use cache::FigureModelCache;
pub use load::load_mesh; // TODO: Don't make this public.

use crate::{
    anim::{
        self, character::CharacterSkeleton, object::ObjectSkeleton, quadruped::QuadrupedSkeleton,
        quadrupedmedium::QuadrupedMediumSkeleton, Animation, Skeleton,
    },
    render::{Consts, FigureBoneData, FigureLocals, Globals, Light, Renderer},
    scene::camera::{Camera, CameraMode},
};
use client::Client;
use common::{
    comp::{self, Body},
    terrain::TerrainChunkSize,
    vol::VolSize,
};
use hashbrown::HashMap;
use log::debug;
use specs::{Entity as EcsEntity, Join};
use vek::*;

const DAMAGE_FADE_COEFFICIENT: f64 = 5.0;

pub struct FigureMgr {
    model_cache: FigureModelCache,
    character_states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
    quadruped_states: HashMap<EcsEntity, FigureState<QuadrupedSkeleton>>,
    quadruped_medium_states: HashMap<EcsEntity, FigureState<QuadrupedMediumSkeleton>>,
    object_states: HashMap<EcsEntity, FigureState<ObjectSkeleton>>,
}

impl FigureMgr {
    pub fn new() -> Self {
        Self {
            model_cache: FigureModelCache::new(),
            character_states: HashMap::new(),
            quadruped_states: HashMap::new(),
            quadruped_medium_states: HashMap::new(),
            object_states: HashMap::new(),
        }
    }

    pub fn clean(&mut self, tick: u64) {
        self.model_cache.clean(tick);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        let time = client.state().get_time();
        let tick = client.get_tick();
        let ecs = client.state().ecs();
        let view_distance = client.view_distance().unwrap_or(1);
        let dt = client.state().get_delta_time();
        // Get player position.
        let player_pos = ecs
            .read_storage::<comp::Pos>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        for (entity, pos, vel, ori, scale, body, animation_info, stats) in (
            &ecs.entities(),
            &ecs.read_storage::<comp::Pos>(),
            &ecs.read_storage::<comp::Vel>(),
            &ecs.read_storage::<comp::Ori>(),
            ecs.read_storage::<comp::Scale>().maybe(),
            &ecs.read_storage::<comp::Body>(),
            ecs.read_storage::<comp::AnimationInfo>().maybe(),
            ecs.read_storage::<comp::Stats>().maybe(),
        )
            .join()
        {
            // Don't process figures outside the vd
            let vd_frac = (pos.0 - player_pos)
                .map2(TerrainChunkSize::SIZE, |d, sz| d.abs() as f32 / sz as f32)
                .magnitude()
                / view_distance as f32;
            // Keep from re-adding/removing entities on the border of the vd
            if vd_frac > 1.2 {
                match body {
                    Body::Humanoid(_) => {
                        self.character_states.remove(&entity);
                    }
                    Body::Quadruped(_) => {
                        self.quadruped_states.remove(&entity);
                    }
                    Body::QuadrupedMedium(_) => {
                        self.quadruped_medium_states.remove(&entity);
                    }
                    Body::Object(_) => {
                        self.object_states.remove(&entity);
                    }
                }
                continue;
            } else if vd_frac > 1.0 {
                continue;
            }

            // Change in health as color!
            let col = stats
                .and_then(|stats| stats.health.last_change)
                .map(|(_, time, _)| {
                    Rgba::broadcast(1.0)
                        + Rgba::new(0.0, -1.0, -1.0, 0.0)
                            .map(|c| (c / (1.0 + DAMAGE_FADE_COEFFICIENT * time)) as f32)
                })
                .unwrap_or(Rgba::broadcast(1.0));

            let scale = scale.map(|s| s.0).unwrap_or(1.0);

            let skeleton_attr = &self
                .model_cache
                .get_or_create_model(renderer, *body, tick)
                .1;

            match body {
                Body::Humanoid(_) => {
                    let state = self
                        .character_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, CharacterSkeleton::new()));

                    let animation_info = match animation_info {
                        Some(a_i) => a_i,
                        None => continue,
                    };

                    let target_skeleton = match animation_info.animation {
                        comp::Animation::Idle => anim::character::IdleAnimation::update_skeleton(
                            state.skeleton_mut(),
                            time,
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Run => anim::character::RunAnimation::update_skeleton(
                            state.skeleton_mut(),
                            (vel.0.magnitude(), time),
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Jump => anim::character::JumpAnimation::update_skeleton(
                            state.skeleton_mut(),
                            time,
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Attack => {
                            anim::character::AttackAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_info.time,
                                skeleton_attr,
                            )
                        }
                        comp::Animation::Block => anim::character::BlockAnimation::update_skeleton(
                            state.skeleton_mut(),
                            time,
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Cjump => anim::character::CjumpAnimation::update_skeleton(
                            state.skeleton_mut(),
                            time,
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Roll => anim::character::RollAnimation::update_skeleton(
                            state.skeleton_mut(),
                            time,
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Crun => anim::character::CrunAnimation::update_skeleton(
                            state.skeleton_mut(),
                            (vel.0.magnitude(), time),
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Cidle => anim::character::CidleAnimation::update_skeleton(
                            state.skeleton_mut(),
                            time,
                            animation_info.time,
                            skeleton_attr,
                        ),
                        comp::Animation::Gliding => {
                            anim::character::GlidingAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_info.time,
                                skeleton_attr,
                            )
                        }
                    };

                    state.skeleton.interpolate(&target_skeleton, dt);
                    state.update(renderer, pos.0, ori.0, scale, col, dt);
                }
                Body::Quadruped(_) => {
                    let state = self
                        .quadruped_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, QuadrupedSkeleton::new()));

                    let animation_info = match animation_info {
                        Some(a_i) => a_i,
                        None => continue,
                    };

                    let target_skeleton = match animation_info.animation {
                        comp::Animation::Run | comp::Animation::Crun => {
                            anim::quadruped::RunAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_info.time,
                                skeleton_attr,
                            )
                        }
                        comp::Animation::Idle | comp::Animation::Cidle => {
                            anim::quadruped::IdleAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_info.time,
                                skeleton_attr,
                            )
                        }
                        comp::Animation::Jump | comp::Animation::Cjump => {
                            anim::quadruped::JumpAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_info.time,
                                skeleton_attr,
                            )
                        }

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_skeleton, dt);
                    state.update(renderer, pos.0, ori.0, scale, col, dt);
                }
                Body::QuadrupedMedium(_) => {
                    let state = self
                        .quadruped_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedMediumSkeleton::new())
                        });

                    let animation_info = match animation_info {
                        Some(a_i) => a_i,
                        None => continue,
                    };

                    let target_skeleton = match animation_info.animation {
                        comp::Animation::Run | comp::Animation::Crun => {
                            anim::quadrupedmedium::RunAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_info.time,
                                skeleton_attr,
                            )
                        }
                        comp::Animation::Idle | comp::Animation::Cidle => {
                            anim::quadrupedmedium::IdleAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_info.time,
                                skeleton_attr,
                            )
                        }
                        comp::Animation::Jump | comp::Animation::Cjump => {
                            anim::quadrupedmedium::JumpAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_info.time,
                                skeleton_attr,
                            )
                        }

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_skeleton, dt);
                    state.update(renderer, pos.0, ori.0, scale, col, dt);
                }
                Body::Object(_) => {
                    let state = self
                        .object_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, ObjectSkeleton::new()));

                    state.skeleton = state.skeleton_mut().clone();
                    state.update(renderer, pos.0, ori.0, scale, col, dt);
                }
            }
        }

        // Clear states that have dead entities.
        self.character_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.quadruped_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.quadruped_medium_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.object_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        client: &mut Client,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        camera: &Camera,
    ) {
        let tick = client.get_tick();
        let ecs = client.state().ecs();

        let frustum = camera.frustum(client);

        for (entity, _, _, _, body, _, _) in (
            &ecs.entities(),
            &ecs.read_storage::<comp::Pos>(),
            &ecs.read_storage::<comp::Vel>(),
            &ecs.read_storage::<comp::Ori>(),
            &ecs.read_storage::<comp::Body>(),
            ecs.read_storage::<comp::Stats>().maybe(),
            ecs.read_storage::<comp::Scale>().maybe(),
        )
            .join()
            // Don't render figures outside of frustum (camera viewport, max draw distance is farplane)
            .filter(|(_, pos, _, _, _, _, scale)| {
                frustum.sphere_intersecting(
                    &pos.0.x,
                    &pos.0.y,
                    &pos.0.z,
                    &(scale.unwrap_or(&comp::Scale(1.0)).0 * 2.0),
                )
            })
            // Don't render dead entities
            .filter(|(_, _, _, _, _, stats, _)| stats.map_or(true, |s| !s.is_dead))
        {
            if let Some((locals, bone_consts)) = match body {
                Body::Humanoid(_) => self
                    .character_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::Quadruped(_) => self
                    .quadruped_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::QuadrupedMedium(_) => self
                    .quadruped_medium_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::Object(_) => self
                    .object_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
            } {
                let model = &self
                    .model_cache
                    .get_or_create_model(renderer, *body, tick)
                    .0;

                // Don't render the player's body while in first person mode
                if camera.get_mode() == CameraMode::FirstPerson
                    && client
                        .state()
                        .read_storage::<comp::Body>()
                        .get(client.entity())
                        .is_some()
                    && entity == client.entity()
                {
                    continue;
                }

                renderer.render_figure(model, globals, locals, bone_consts, lights);
            } else {
                debug!("Body has no saved figure");
            }
        }
    }
}

pub struct FigureState<S: Skeleton> {
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,
    skeleton: S,
    pos: Vec3<f32>,
    ori: Vec3<f32>,
}

impl<S: Skeleton> FigureState<S> {
    pub fn new(renderer: &mut Renderer, skeleton: S) -> Self {
        Self {
            bone_consts: renderer
                .create_consts(&skeleton.compute_matrices())
                .unwrap(),
            locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
            skeleton,
            pos: Vec3::zero(),
            ori: Vec3::zero(),
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        pos: Vec3<f32>,
        ori: Vec3<f32>,
        scale: f32,
        col: Rgba<f32>,
        dt: f32,
    ) {
        // Update interpolation values
        if self.pos.distance_squared(pos) < 64.0 * 64.0 {
            self.pos = Lerp::lerp(self.pos, pos, 15.0 * dt);
            self.ori = Slerp::slerp(self.ori, ori, 7.5 * dt);
        } else {
            self.pos = pos;
            self.ori = ori;
        }

        let mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(self.pos)
            * Mat4::rotation_z(-ori.x.atan2(ori.y))
            * Mat4::scaling_3d(Vec3::from(0.8 * scale));

        let locals = FigureLocals::new(mat, col);
        renderer.update_consts(&mut self.locals, &[locals]).unwrap();

        renderer
            .update_consts(&mut self.bone_consts, &self.skeleton.compute_matrices())
            .unwrap();
    }

    pub fn locals(&self) -> &Consts<FigureLocals> {
        &self.locals
    }

    pub fn bone_consts(&self) -> &Consts<FigureBoneData> {
        &self.bone_consts
    }

    pub fn skeleton_mut(&mut self) -> &mut S {
        &mut self.skeleton
    }
}
