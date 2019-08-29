use super::load::*;
use crate::{
    anim::SkeletonAttr,
    render::{FigurePipeline, Mesh, Model, Renderer},
};
use common::{
    assets::watch::ReloadIndicator,
    comp::{item::Tool, Body},
};
use hashbrown::HashMap;

pub struct FigureModelCache {
    models: HashMap<Body, ((Model<FigurePipeline>, SkeletonAttr), u64)>,
    manifest_indicator: ReloadIndicator,
}

impl FigureModelCache {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            manifest_indicator: ReloadIndicator::new(),
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
                            let humanoid_head_spec =
                                HumHeadSpec::load_watched(&mut self.manifest_indicator);
                            let bone_meshes = match body {
                                Body::Humanoid(body) => [
                                    Some(humanoid_head_spec.mesh_head(
                                        body.race,
                                        body.body_type,
                                        body.hair_color,
                                        body.hair_style,
                                        body.beard,
                                        body.eye_color,
                                        body.skin,
                                        body.eyebrows,
                                        body.accessory,
                                    )),
                                    Some(mesh_chest(body.chest)),
                                    Some(mesh_belt(body.belt)),
                                    Some(mesh_pants(body.pants)),
                                    Some(mesh_left_hand(body.hand)),
                                    Some(mesh_right_hand(body.hand)),
                                    Some(mesh_left_foot(body.foot)),
                                    Some(mesh_right_foot(body.foot)),
                                    Some(mesh_weapon(Tool::Hammer)), // TODO: Inventory
                                    Some(mesh_left_shoulder(body.shoulder)),
                                    Some(mesh_right_shoulder(body.shoulder)),
                                    Some(mesh_draw()),
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                                Body::Quadruped(body) => [
                                    Some(mesh_pig_head(body.head)),
                                    Some(mesh_pig_chest(body.chest)),
                                    Some(mesh_pig_leg_lf(body.leg_l)),
                                    Some(mesh_pig_leg_rf(body.leg_r)),
                                    Some(mesh_pig_leg_lb(body.leg_l)),
                                    Some(mesh_pig_leg_rb(body.leg_r)),
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
                                    Some(mesh_wolf_head_upper(body.head_upper)),
                                    Some(mesh_wolf_jaw(body.jaw)),
                                    Some(mesh_wolf_head_lower(body.head_lower)),
                                    Some(mesh_wolf_tail(body.tail)),
                                    Some(mesh_wolf_torso_back(body.torso_back)),
                                    Some(mesh_wolf_torso_mid(body.torso_mid)),
                                    Some(mesh_wolf_ears(body.ears)),
                                    Some(mesh_wolf_foot_lf(body.foot_lf)),
                                    Some(mesh_wolf_foot_rf(body.foot_rf)),
                                    Some(mesh_wolf_foot_lb(body.foot_lb)),
                                    Some(mesh_wolf_foot_rb(body.foot_rb)),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                                Body::Object(object) => [
                                    Some(mesh_object(object)),
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
        // Check for reloaded manifests
        // TODO: maybe do this in a different function, maintain?
        if self.manifest_indicator.reloaded() {
            self.models.clear();
        }
        // TODO: Don't hard-code this.
        self.models
            .retain(|_, (_, last_used)| *last_used + 60 > tick);
    }
}
