use super::load::*;
use crate::{
    anim::SkeletonAttr,
    render::{FigurePipeline, Mesh, Model, Renderer},
};
use common::comp::{item::Tool, Body};
use hashbrown::HashMap;

pub struct FigureModelCache {
    models: HashMap<Body, ((Model<FigurePipeline>, SkeletonAttr), u64)>,
}

impl FigureModelCache {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn get_or_create_model(
        &mut self,
        renderer: &mut Renderer,
        body: Body,
        tick: u64,
    ) -> &(Model<FigurePipeline>, SkeletonAttr) {
        match self.models.get_mut(&body) {
            Some((_model, last_used)) => {
                *last_used = tick;
            }
            None => {
                self.models.insert(
                    body,
                    (
                        {
                            let bone_meshes = match body {
                                Body::Humanoid(body) => [
                                    Some(load_head(body.race, body.body_type)),
                                    Some(load_chest(body.chest)),
                                    Some(load_belt(body.belt)),
                                    Some(load_pants(body.pants)),
                                    Some(load_left_hand(body.hand)),
                                    Some(load_right_hand(body.hand)),
                                    Some(load_left_foot(body.foot)),
                                    Some(load_right_foot(body.foot)),
                                    Some(load_weapon(Tool::Hammer)), // TODO: Inventory
                                    Some(load_left_shoulder(body.shoulder)),
                                    Some(load_right_shoulder(body.shoulder)),
                                    Some(load_draw()),
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                                Body::Quadruped(body) => [
                                    Some(load_pig_head(body.head)),
                                    Some(load_pig_chest(body.chest)),
                                    Some(load_pig_leg_lf(body.leg_l)),
                                    Some(load_pig_leg_rf(body.leg_r)),
                                    Some(load_pig_leg_lb(body.leg_l)),
                                    Some(load_pig_leg_rb(body.leg_r)),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                                Body::QuadrupedMedium(body) => [
                                    Some(load_wolf_head_upper(body.head_upper)),
                                    Some(load_wolf_jaw(body.jaw)),
                                    Some(load_wolf_head_lower(body.head_lower)),
                                    Some(load_wolf_tail(body.tail)),
                                    Some(load_wolf_torso_back(body.torso_back)),
                                    Some(load_wolf_torso_mid(body.torso_mid)),
                                    Some(load_wolf_ears(body.ears)),
                                    Some(load_wolf_foot_lf(body.foot_lf)),
                                    Some(load_wolf_foot_rf(body.foot_rf)),
                                    Some(load_wolf_foot_lb(body.foot_lb)),
                                    Some(load_wolf_foot_rb(body.foot_rb)),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                                Body::Object(object) => [
                                    Some(load_object(object)),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                            };

                            let skeleton_attr = match body {
                                Body::Humanoid(body) => SkeletonAttr::from(&body),
                                _ => SkeletonAttr::default(),
                            };

                            let mut mesh = Mesh::new();
                            bone_meshes
                                .iter()
                                .enumerate()
                                .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i, bm)))
                                .for_each(|(i, bone_mesh)| {
                                    mesh.push_mesh_map(bone_mesh, |vert| {
                                        vert.with_bone_idx(i as u8)
                                    })
                                });

                            (renderer.create_model(&mesh).unwrap(), skeleton_attr)
                        },
                        tick,
                    ),
                );
            }
        }

        &self.models[&body].0
    }

    pub fn clean(&mut self, tick: u64) {
        // TODO: Don't hard-code this.
        self.models
            .retain(|_, (_, last_used)| *last_used + 60 > tick);
    }
}
