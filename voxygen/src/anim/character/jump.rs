use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: f64,
        anim_time: f64,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let wave = (anim_time as f32 * 14.0).sin();
        let wave_slow = (anim_time as f32 * 7.0).sin();
        let wave_stop = (anim_time as f32 * 4.5).min(PI / 2.0).sin();
        let wave_stop_alt = (anim_time as f32 * 5.0).min(PI / 2.0).sin();

        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            0.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 15.0,
        );
        next.head.ori = Quaternion::rotation_x(0.25 + wave_stop * 0.1 + wave_slow * 0.04);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 8.0);
        next.chest.ori = Quaternion::rotation_z(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 6.0);
        next.belt.ori = Quaternion::rotation_z(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 3.0);
        next.shorts.ori = Quaternion::rotation_z(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -8.0,
            0.0 + wave_stop * 3.8,
            7.0 + wave_stop * 3.2 - wave * 0.4,
        );
        next.l_hand.ori = Quaternion::rotation_x(wave_stop_alt * 0.6);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            8.0,
            0.0 + wave_stop * -3.8,
            7.0 + wave_stop * 3.2 - wave * 0.4,
        );
        next.r_hand.ori = Quaternion::rotation_x(-wave_stop_alt * 0.6);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 1.0, 6.0);
        next.l_foot.ori = Quaternion::rotation_x(wave_stop * -1.2 - wave_slow * 0.2);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, -1.0, 6.0);
        next.r_foot.ori = Quaternion::rotation_x(wave_stop * 1.2 + wave_slow * 0.2);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.weapon.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-10.0, -3.2, 2.5);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.04;

        next.r_shoulder.offset = Vec3::new(0.0, -3.2, 2.5);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.04;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.left_equip.offset = Vec3::new(0.0, 0.0, 5.0) / 11.0;
        next.left_equip.ori = Quaternion::rotation_x(0.0);;
        next.left_equip.scale = Vec3::one() * 0.0;

        next.right_equip.offset = Vec3::new(0.0, 0.0, 5.0) / 11.0;
        next.right_equip.ori = Quaternion::rotation_x(0.0);;
        next.right_equip.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.0) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(-0.2);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
