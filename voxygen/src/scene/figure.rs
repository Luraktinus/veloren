use crate::{
    anim::{
        self, character::CharacterSkeleton, object::ObjectSkeleton, quadruped::QuadrupedSkeleton,
        quadrupedmedium::QuadrupedMediumSkeleton, Animation, Skeleton, SkeletonAttr,
    },
    mesh::Meshable,
    render::{
        Consts, FigureBoneData, FigureLocals, FigurePipeline, Globals, Light, Mesh, Model, Renderer,
    },
    scene::camera::{Camera, CameraMode},
};
use client::Client;
use common::{
    assets,
    comp::{self, humanoid, item::Tool, object, quadruped, quadruped_medium, Body},
    figure::Segment,
    terrain::TerrainChunkSize,
    vol::VolSize,
};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use log::debug;
use specs::{Entity as EcsEntity, Join};
use std::f32;
use vek::*;

const DAMAGE_FADE_COEFFICIENT: f64 = 5.0;

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

fn load_segment(mesh_name: &str) -> Segment {
    let full_specifier: String = ["voxygen.voxel.", mesh_name].concat();
    Segment::from(assets::load_expect::<DotVoxData>(full_specifier.as_str()).as_ref())
}
// TODO: Don't make this public.
pub fn load_mesh(mesh_name: &str, position: Vec3<f32>) -> Mesh<FigurePipeline> {
    Meshable::<FigurePipeline, FigurePipeline>::generate_mesh(&load_segment(mesh_name), position).0
}

fn load_head(race: humanoid::Race, body_type: humanoid::BodyType) -> Mesh<FigurePipeline> {
    use humanoid::{BodyType, Race};

    let (name, offset) = match (race, body_type) {
        // z-value should be 0.25 of the total z
        (Race::Human, BodyType::Male) => {
            ("figure.head.head_human_male", Vec3::new(-7.0, -5.0, -2.25))
        }
        (Race::Human, BodyType::Female) => (
            "figure.head.head_human_female",
            Vec3::new(-7.0, -7.5, -3.25),
        ),
        (Race::Elf, BodyType::Male) => ("figure.head.head_elf_male", Vec3::new(-8.0, -5.0, -2.25)),
        (Race::Elf, BodyType::Female) => {
            ("figure.head.head_elf_female", Vec3::new(-8.0, -5.5, -3.0))
        }
        (Race::Dwarf, BodyType::Male) => {
            ("figure.head.head_dwarf_male", Vec3::new(-6.0, -5.0, -12.5))
        }
        (Race::Dwarf, BodyType::Female) => (
            "figure.head.head_dwarf_female",
            Vec3::new(-6.0, -6.0, -9.25),
        ),
        (Race::Orc, BodyType::Male) => ("figure.head.head_orc_male", Vec3::new(-8.0, -5.0, -2.50)),
        (Race::Orc, BodyType::Female) => {
            ("figure.head.head_orc_female", Vec3::new(-8.0, -8.0, -3.5))
        }
        (Race::Undead, BodyType::Male) => {
            ("figure.head.head_undead_male", Vec3::new(-5.5, -5.0, -2.5))
        }
        (Race::Undead, BodyType::Female) => (
            "figure.head.head_undead_female",
            Vec3::new(-6.0, -5.0, -2.5),
        ),
        (Race::Danari, BodyType::Male) => {
            ("figure.head.head_danari_male", Vec3::new(-9.0, -5.0, -2.75))
        }
        (Race::Danari, BodyType::Female) => (
            "figure.head.head_danari_female",
            Vec3::new(-9.0, -7.5, -3.0),
        ),
    };
    load_mesh(name, offset)
}
// loads models with different offsets
//    fn load_beard(beard: Beard) -> Mesh<FigurePipeline> {
//        let (name, offset) = match beard {
//            Beard::None => ("figure/body/empty", Vec3::new(0.0, 0.0, 0.0)),
//            Beard::Human1 => ("figure/empty", Vec3::new(0.0, 0.0, 0.0)),
//        };
//        load_mesh(name, offset)
//    }

fn load_chest(chest: humanoid::Chest) -> Mesh<FigurePipeline> {
    use humanoid::Chest;
    let color = match chest {
        Chest::Brown => (125, 53, 0),
        Chest::Dark => (0, 38, 43),
        Chest::Green => (0, 255, 34),
        Chest::Orange => (255, 106, 0),
        Chest::Blue => (0, 38, 255),
    };

    let bare_chest = load_segment("figure.body.chest");
    let chest_armor = load_segment("armor.chest.generic");
    let chest = bare_chest.union(&chest_armor.chromify(Rgb::from(color)), Vec3::new(0, 0, 0));

    Meshable::<FigurePipeline, FigurePipeline>::generate_mesh(&chest, Vec3::new(-6.0, -3.5, 0.0)).0
}

fn load_belt(belt: humanoid::Belt) -> Mesh<FigurePipeline> {
    use humanoid::Belt;

    load_mesh(
        match belt {
            //Belt::Default => "figure/body/belt_male",
            Belt::Dark => "armor.belt.belt_dark",
        },
        Vec3::new(-5.0, -3.5, 0.0),
    )
}

fn load_pants(pants: humanoid::Pants) -> Mesh<FigurePipeline> {
    use humanoid::Pants;

    load_mesh(
        match pants {
            Pants::Blue => "armor.pants.pants_blue",
            Pants::Brown => "armor.pants.pants_brown",
            Pants::Dark => "armor.pants.pants_dark",
            Pants::Green => "armor.pants.pants_green",
            Pants::Orange => "armor.pants.pants_orange",
        },
        Vec3::new(-5.0, -3.5, 0.0),
    )
}

fn load_left_hand(hand: humanoid::Hand) -> Mesh<FigurePipeline> {
    load_mesh(
        match hand {
            humanoid::Hand::Default => "figure.body.hand",
        },
        Vec3::new(-2.0, -2.5, -2.0),
    )
}

fn load_right_hand(hand: humanoid::Hand) -> Mesh<FigurePipeline> {
    load_mesh(
        match hand {
            humanoid::Hand::Default => "figure.body.hand",
        },
        Vec3::new(-2.0, -2.5, -2.0),
    )
}

fn load_left_foot(foot: humanoid::Foot) -> Mesh<FigurePipeline> {
    use humanoid::Foot;

    load_mesh(
        match foot {
            Foot::Dark => "armor.foot.foot_dark",
        },
        Vec3::new(-2.5, -3.5, -9.0),
    )
}

fn load_right_foot(foot: humanoid::Foot) -> Mesh<FigurePipeline> {
    use humanoid::Foot;

    load_mesh(
        match foot {
            Foot::Dark => "armor.foot.foot_dark",
        },
        Vec3::new(-2.5, -3.5, -9.0),
    )
}

fn load_weapon(weapon: Tool) -> Mesh<FigurePipeline> {
    let (name, offset) = match weapon {
        Tool::Sword => ("weapon.sword.rusty_2h", Vec3::new(-1.5, -6.5, -4.0)),
        Tool::Axe => ("weapon.axe.rusty_2h", Vec3::new(-1.5, -6.5, -4.0)),
        Tool::Hammer => ("weapon.hammer.rusty_2h", Vec3::new(-2.5, -5.5, -4.0)),
        Tool::Daggers => ("weapon.hammer.rusty_2h", Vec3::new(-2.5, -5.5, -4.0)),
        Tool::SwordShield => ("weapon.axe.rusty_2h", Vec3::new(-2.5, -6.5, -2.0)),
        Tool::Bow => ("weapon.hammer.rusty_2h", Vec3::new(-2.5, -5.5, -4.0)),
        Tool::Staff => ("weapon.axe.rusty_2h", Vec3::new(-2.5, -6.5, -2.0)),
    };
    load_mesh(name, offset)
}

fn load_left_shoulder(shoulder: humanoid::Shoulder) -> Mesh<FigurePipeline> {
    load_mesh(
        match shoulder {
            humanoid::Shoulder::None => "figure.empty",
            humanoid::Shoulder::Brown1 => "armor.shoulder.shoulder_l_brown",
        },
        Vec3::new(-2.5, -3.5, -1.5),
    )
}

fn load_right_shoulder(shoulder: humanoid::Shoulder) -> Mesh<FigurePipeline> {
    load_mesh(
        match shoulder {
            humanoid::Shoulder::None => "figure.empty",
            humanoid::Shoulder::Brown1 => "armor.shoulder.shoulder_r_brown",
        },
        Vec3::new(-2.5, -3.5, -1.5),
    )
}

// TODO: Inventory
fn load_draw() -> Mesh<FigurePipeline> {
    load_mesh("object.glider", Vec3::new(-26.0, -26.0, -5.0))
}

//fn load_right_equip(hand: humanoid::Hand) -> Mesh<FigurePipeline> {
//    load_mesh(
//        match hand {
//            humanoid::Hand::Default => "figure/body/hand",
//        },
//        Vec3::new(-2.0, -2.5, -5.0),
//    )
//}

/////////
fn load_pig_head(head: quadruped::Head) -> Mesh<FigurePipeline> {
    load_mesh(
        match head {
            quadruped::Head::Default => "npc.pig_purple.pig_head",
        },
        Vec3::new(-6.0, 4.5, 3.0),
    )
}

fn load_pig_chest(chest: quadruped::Chest) -> Mesh<FigurePipeline> {
    load_mesh(
        match chest {
            quadruped::Chest::Default => "npc.pig_purple.pig_chest",
        },
        Vec3::new(-5.0, 4.5, 0.0),
    )
}

fn load_pig_leg_lf(leg_l: quadruped::LegL) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_l {
            quadruped::LegL::Default => "npc.pig_purple.pig_leg_l",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}

fn load_pig_leg_rf(leg_r: quadruped::LegR) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_r {
            quadruped::LegR::Default => "npc.pig_purple.pig_leg_r",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}

fn load_pig_leg_lb(leg_l: quadruped::LegL) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_l {
            quadruped::LegL::Default => "npc.pig_purple.pig_leg_l",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}

fn load_pig_leg_rb(leg_r: quadruped::LegR) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_r {
            quadruped::LegR::Default => "npc.pig_purple.pig_leg_r",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}
//////
fn load_wolf_head_upper(upper_head: quadruped_medium::HeadUpper) -> Mesh<FigurePipeline> {
    load_mesh(
        match upper_head {
            quadruped_medium::HeadUpper::Default => "npc.wolf.wolf_head_upper",
        },
        Vec3::new(-7.0, -6.0, -5.5),
    )
}

fn load_wolf_jaw(jaw: quadruped_medium::Jaw) -> Mesh<FigurePipeline> {
    load_mesh(
        match jaw {
            quadruped_medium::Jaw::Default => "npc.wolf.wolf_jaw",
        },
        Vec3::new(-3.0, -3.0, -2.5),
    )
}

fn load_wolf_head_lower(head_lower: quadruped_medium::HeadLower) -> Mesh<FigurePipeline> {
    load_mesh(
        match head_lower {
            quadruped_medium::HeadLower::Default => "npc.wolf.wolf_head_lower",
        },
        Vec3::new(-7.0, -6.0, -5.5),
    )
}

fn load_wolf_tail(tail: quadruped_medium::Tail) -> Mesh<FigurePipeline> {
    load_mesh(
        match tail {
            quadruped_medium::Tail::Default => "npc.wolf.wolf_tail",
        },
        Vec3::new(-2.0, -12.0, -5.0),
    )
}

fn load_wolf_torso_back(torso_back: quadruped_medium::TorsoBack) -> Mesh<FigurePipeline> {
    load_mesh(
        match torso_back {
            quadruped_medium::TorsoBack::Default => "npc.wolf.wolf_torso_back",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn load_wolf_torso_mid(torso_mid: quadruped_medium::TorsoMid) -> Mesh<FigurePipeline> {
    load_mesh(
        match torso_mid {
            quadruped_medium::TorsoMid::Default => "npc.wolf.wolf_torso_mid",
        },
        Vec3::new(-8.0, -5.5, -6.0),
    )
}

fn load_wolf_ears(ears: quadruped_medium::Ears) -> Mesh<FigurePipeline> {
    load_mesh(
        match ears {
            quadruped_medium::Ears::Default => "npc.wolf.wolf_ears",
        },
        Vec3::new(-4.0, -1.0, -1.0),
    )
}

fn load_wolf_foot_lf(foot_lf: quadruped_medium::FootLF) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_lf {
            quadruped_medium::FootLF::Default => "npc.wolf.wolf_foot_lf",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

fn load_wolf_foot_rf(foot_rf: quadruped_medium::FootRF) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_rf {
            quadruped_medium::FootRF::Default => "npc.wolf.wolf_foot_rf",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

fn load_wolf_foot_lb(foot_lb: quadruped_medium::FootLB) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_lb {
            quadruped_medium::FootLB::Default => "npc.wolf.wolf_foot_lb",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

fn load_wolf_foot_rb(foot_rb: quadruped_medium::FootRB) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_rb {
            quadruped_medium::FootRB::Default => "npc.wolf.wolf_foot_rb",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

fn load_object(obj: object::Body) -> Mesh<FigurePipeline> {
    let (name, offset) = match obj {
        object::Body::Bomb => ("object.bomb", Vec3::new(-5.5, -5.5, 0.0)),
        object::Body::Scarecrow => ("object.scarecrow", Vec3::new(-9.5, -4.0, 0.0)),
        object::Body::Cauldron => ("object.cauldron", Vec3::new(-10.0, -10.0, 0.0)),
        object::Body::ChestVines => ("object.chest_vines", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::Chest => ("object.chest", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::ChestDark => ("object.chest_dark", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::ChestDemon => ("object.chest_demon", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::ChestGold => ("object.chest_gold", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::ChestLight => ("object.chest_light", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::ChestOpen => ("object.chest_open", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::ChestSkull => ("object.chest_skull", Vec3::new(-7.5, -6.0, 0.0)),
        object::Body::Pumpkin => ("object.pumpkin", Vec3::new(-5.5, -4.0, 0.0)),
        object::Body::Pumpkin2 => ("object.pumpkin_2", Vec3::new(-5.0, -4.0, 0.0)),
        object::Body::Pumpkin3 => ("object.pumpkin_3", Vec3::new(-5.0, -4.0, 0.0)),
        object::Body::Pumpkin4 => ("object.pumpkin_4", Vec3::new(-5.0, -4.0, 0.0)),
        object::Body::Pumpkin5 => ("object.pumpkin_5", Vec3::new(-4.0, -5.0, 0.0)),
        object::Body::Campfire => ("object.campfire", Vec3::new(-9.0, -10.0, 0.0)),
        object::Body::LanternGround => ("object.lantern_ground", Vec3::new(-3.5, -3.5, 0.0)),
        object::Body::LanternGroundOpen => {
            ("object.lantern_ground_open", Vec3::new(-3.5, -3.5, 0.0))
        }
        object::Body::LanternStanding => ("object.lantern_standing", Vec3::new(-7.5, -3.5, 0.0)),
        object::Body::LanternStanding2 => {
            ("object.lantern_standing_2", Vec3::new(-11.5, -3.5, 0.0))
        }
        object::Body::PotionRed => ("object.potion_red", Vec3::new(-2.0, -2.0, 0.0)),
        object::Body::PotionBlue => ("object.potion_blue", Vec3::new(-2.0, -2.0, 0.0)),
        object::Body::PotionGreen => ("object.potion_green", Vec3::new(-2.0, -2.0, 0.0)),
        object::Body::Crate => ("object.crate", Vec3::new(-7.0, -7.0, 0.0)),
        object::Body::Tent => ("object.tent", Vec3::new(-18.5, -19.5, 0.0)),
        object::Body::WindowSpooky => ("object.window_spooky", Vec3::new(-15.0, -1.5, -1.0)),
        object::Body::DoorSpooky => ("object.door_spooky", Vec3::new(-15.0, -4.5, 0.0)),
        object::Body::Table => ("object.table", Vec3::new(-12.0, -8.0, 0.0)),
        object::Body::Table2 => ("object.table_2", Vec3::new(-8.0, -8.0, 0.0)),
        object::Body::Table3 => ("object.table_3", Vec3::new(-10.0, -10.0, 0.0)),
        object::Body::Drawer => ("object.drawer", Vec3::new(-11.0, -7.5, 0.0)),
        object::Body::BedBlue => ("object.bed_human_blue", Vec3::new(-11.0, -15.0, 0.0)),
        object::Body::Anvil => ("object.anvil", Vec3::new(-3.0, -7.0, 0.0)),
        object::Body::Gravestone => ("object.gravestone", Vec3::new(-5.0, -2.0, 0.0)),
        object::Body::Gravestone2 => ("object.gravestone_2", Vec3::new(-8.5, -3.0, 0.0)),
        object::Body::Chair => ("object.chair", Vec3::new(-5.0, -4.5, 0.0)),
        object::Body::Chair2 => ("object.chair_2", Vec3::new(-5.0, -4.5, 0.0)),
        object::Body::Chair3 => ("object.chair_3", Vec3::new(-5.0, -4.5, 0.0)),
        object::Body::Bench => ("object.bench", Vec3::new(-8.8, -5.0, 0.0)),
        object::Body::Carpet => ("object.carpet", Vec3::new(-14.0, -14.0, -0.5)),
        object::Body::Bedroll => ("object.bedroll", Vec3::new(-11.0, -19.5, -0.5)),
        object::Body::CarpetHumanRound => {
            ("object.carpet_human_round", Vec3::new(-14.0, -14.0, -0.5))
        }
        object::Body::CarpetHumanSquare => {
            ("object.carpet_human_square", Vec3::new(-13.5, -14.0, -0.5))
        }
        object::Body::CarpetHumanSquare2 => (
            "object.carpet_human_square_2",
            Vec3::new(-13.5, -14.0, -0.5),
        ),
        object::Body::CarpetHumanSquircle => (
            "object.carpet_human_squircle",
            Vec3::new(-21.0, -21.0, -0.5),
        ),
        object::Body::Pouch => ("object.pouch", Vec3::new(-5.5, -4.5, 0.0)),
    };
    load_mesh(name, offset)
}

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
