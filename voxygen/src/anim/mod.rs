pub mod character;
pub mod fixture;
pub mod quadruped;
pub mod quadrupedmedium;

use crate::render::FigureBoneData;
use vek::*;
use common::comp::actor::{HumanoidBody, Head, Weapon};

#[derive(Copy, Clone)]
pub struct Bone {
    pub offset: Vec3<f32>,
    pub ori: Quaternion<f32>,
    pub scale: Vec3<f32>,
}

impl Bone {
    pub fn default() -> Self {
        Self {
            offset: Vec3::zero(),
            ori: Quaternion::identity(),
            scale: Vec3::broadcast(1.0 / 11.0),
        }
    }

    pub fn compute_base_matrix(&self) -> Mat4<f32> {
        Mat4::<f32>::translation_3d(self.offset)
            * Mat4::scaling_3d(self.scale)
            * Mat4::from(self.ori)
    }

    /// Change the current bone to be more like `target`.
    fn interpolate(&mut self, target: &Bone) {
        // TODO: Make configurable.
        let factor = 0.3;
        self.offset += (target.offset - self.offset) * factor;
        self.ori = vek::ops::Slerp::slerp(self.ori, target.ori, factor);
        self.scale += (target.scale - self.scale) * factor;
    }
}

pub trait Skeleton: Send + Sync + 'static {
    fn compute_matrices(&self) -> [FigureBoneData; 16];

    /// Change the current skeleton to be more like `target`.
    fn interpolate(&mut self, target: &Self);
}

pub struct SkeletonAttr {
    scaler: f32,
    head_scale: f32,
    neck_height: f32,
    neck_forward: f32,
    weapon_x: f32,
    weapon_y: f32,

}


impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            scaler: 1.0,
            head_scale: 1.0,
            neck_height: 1.0,
            neck_forward: 1.0,
            weapon_x: 1.0,
            weapon_y: 1.0,
        }
    }
}

impl<'a> From<&'a HumanoidBody> for SkeletonAttr {
    fn from(body: &'a HumanoidBody) -> Self {
        Self {
            scaler: match body.head {
                Head::OrcMale => 1.10,
                Head::OrcFemale => 1.05,
                Head::HumanMale => 1.0, 
                Head::HumanFemale => 0.95,
                Head::ElfMale => 1.05,
                Head::ElfFemale => 1.0,
                Head::DwarfMale => 0.9,
                Head::DwarfFemale => 0.9,
                Head::UndeadMale => 1.0,
                Head::UndeadFemale => 0.95,
                Head::DanariMale => 0.8,
                Head::DanariFemale => 0.78,
                _ => 1.0,
            },
            head_scale: match body.head {
                Head::OrcMale => 0.9,
                Head::OrcFemale => 0.9,
                Head::HumanMale => 1.0, 
                Head::HumanFemale => 1.0,
                Head::ElfMale => 0.9,
                Head::ElfFemale => 0.9,
                Head::DwarfMale => 1.0,
                Head::DwarfFemale => 1.0,
                Head::UndeadMale => 1.0,
                Head::UndeadFemale => 1.0,
                Head::DanariMale => 1.2,
                Head::DanariFemale => 1.2,
                _ => 1.0,
            },
            neck_height: match body.head {
                Head::OrcMale => -1.0,
                Head::OrcFemale => -1.0,
                Head::HumanMale => -1.0, 
                Head::HumanFemale => -2.0,
                Head::ElfMale => -0.5,
                Head::ElfFemale => -1.0,
                Head::DwarfMale => -0.0,
                Head::DwarfFemale => -1.0,
                Head::UndeadMale => -1.0,
                Head::UndeadFemale => -1.0,
                Head::DanariMale => 0.5,
                Head::DanariFemale => -0.5,
                _ => 1.0,
            },
            neck_forward: match body.head {
                Head::DwarfFemale => -1.0,
                Head::HumanMale => 2.0,
                Head::HumanFemale => 0.0,
                Head::OrcMale => 1.0,
                Head::OrcFemale => 1.5, 
                Head::ElfMale => -0.5,
                Head::ElfFemale => 0.0,
                Head::DwarfMale => 2.0,
                Head::UndeadMale => 1.0,
                Head::UndeadFemale => 1.0,
                Head::DanariMale => 0.5,
                Head::DanariFemale => 0.0,
                _ => 1.0,
            },
            weapon_x: match body.weapon {
                Weapon::Sword => 0.0,
                Weapon::Axe => 3.0,
                Weapon::Hammer => 0.0,
                Weapon::SwordShield => 3.0,
                Weapon::Staff => 3.0,
                Weapon::Bow => 0.0,
                Weapon::Daggers => 0.0,


                _ => 1.0,
            },
            weapon_y: match body.weapon {
                Weapon::Sword => 0.0,
                Weapon::Axe => 1.0,
                Weapon::Hammer => -2.0,
                Weapon::SwordShield => 1.0,
                Weapon::Staff => 1.0,
                Weapon::Bow => -2.0,
                Weapon::Daggers => -2.0,


                _ => 1.0,
            },
        }
    }
}

pub trait Animation {
    type Skeleton;
    type Dependency;

    /// Returns a new skeleton that is generated by the animation.
    fn update_skeleton(
        skeleton: &Self::Skeleton,
        dependency: Self::Dependency,
        anim_time: f64,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton;
}
