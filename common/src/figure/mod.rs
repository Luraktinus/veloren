pub mod cell;
pub mod mat_cell;
pub use mat_cell::Material;

use self::cell::Cell;
use self::mat_cell::MatCell;
use crate::{
    util::chromify_srgb,
    vol::{ReadVol, SizedVol, Vox, WriteVol},
    volumes::dyna::Dyna,
};
use dot_vox::DotVoxData;
use vek::*;

/// A type representing a volume that may be part of an animated figure.
///
/// Figures are used to represent things like characters, NPCs, mobs, etc.
pub type Segment = Dyna<Cell, ()>;

impl From<&DotVoxData> for Segment {
    fn from(dot_vox_data: &DotVoxData) -> Self {
        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut segment = Segment::filled(
                Vec3::new(model.size.x, model.size.y, model.size.z),
                Cell::empty(),
                (),
            );

            for voxel in &model.voxels {
                if let Some(&color) = palette.get(voxel.i as usize) {
                    segment
                        .set(
                            Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| i32::from(e)),
                            Cell::new(color),
                        )
                        .unwrap();
                }
            }

            segment
        } else {
            Segment::filled(Vec3::zero(), Cell::empty(), ())
        }
    }
}

impl Segment {
    /// Replaces one cell with another
    pub fn replace(mut self, old: Cell, new: Cell) -> Self {
        for pos in self.iter_positions() {
            if old == *self.get(pos).unwrap() {
                self.set(pos, new).unwrap();
            }
        }

        self
    }
    /// Preserve the luminance of all the colors but set the chomaticity to match the provided color
    // TODO add more advanced recoloring and/or indexed based coloring
    pub fn chromify(mut self, chroma: Rgb<u8>) -> Self {
        let chroma = chroma.map(|e| e as f32 * 255.0);
        for pos in self.iter_positions() {
            let cell = match self.get(pos).unwrap() {
                Cell::Filled(rgb) => Cell::Filled(
                    chromify_srgb(Rgb::from_slice(rgb).map(|e| e as f32 / 255.0), chroma)
                        .map(|e| (e * 255.0) as u8)
                        .into_array(),
                ),
                Cell::Empty => continue,
            };
            self.set(pos, cell).unwrap();
        }

        self
    }
}

// TODO: move
/// A `Dyna` builder that combines Dynas
pub struct DynaUnionizer<V: Vox>(Vec<(Dyna<V, ()>, Vec3<i32>)>);

impl<V: Vox + Copy> DynaUnionizer<V> {
    pub fn new() -> Self {
        DynaUnionizer(Vec::new())
    }
    pub fn add(mut self, dyna: Dyna<V, ()>, offset: Vec3<i32>) -> Self {
        self.0.push((dyna, offset));
        self
    }
    pub fn maybe_add(self, maybe: Option<(Dyna<V, ()>, Vec3<i32>)>) -> Self {
        match maybe {
            Some((dyna, offset)) => self.add(dyna, offset),
            None => self,
        }
    }
    pub fn unify(self) -> (Dyna<V, ()>, Vec3<i32>) {
        if self.0.is_empty() {
            return (Dyna::filled(Vec3::zero(), V::empty(), ()), Vec3::zero());
        }

        // Determine size of the new Dyna
        let mut min_point = self.0[0].1;
        let mut max_point = self.0[0].1 + self.0[0].0.get_size().map(|e| e as i32);
        for (dyna, offset) in self.0.iter().skip(1) {
            let size = dyna.get_size().map(|e| e as i32);
            min_point = min_point.map2(*offset, std::cmp::min);
            max_point = max_point.map2(offset + size, std::cmp::max);
        }
        let new_size = (max_point - min_point).map(|e| e as u32);
        // Allocate new segment
        let mut combined = Dyna::filled(new_size, V::empty(), ());
        // Copy segments into combined
        let origin = min_point.map(|e| e * -1);
        for (dyna, offset) in self.0 {
            for pos in dyna.iter_positions() {
                let vox = dyna.get(pos).unwrap();
                if !vox.is_empty() {
                    combined.set(origin + offset + pos, *vox).unwrap();
                }
            }
        }

        (combined, origin)
    }
}

pub type MatSegment = Dyna<MatCell, ()>;

impl MatSegment {
    pub fn to_segment(&self, map: impl Fn(Material) -> Rgb<u8>) -> Segment {
        let mut vol = Dyna::filled(self.get_size(), Cell::empty(), ());
        for pos in self.iter_positions() {
            let rgb = match self.get(pos).unwrap() {
                MatCell::None => continue,
                MatCell::Mat(mat) => map(*mat),
                MatCell::Normal(rgb) => *rgb,
            };
            vol.set(pos, Cell::new(rgb)).unwrap();
        }
        vol
    }
}

impl From<&DotVoxData> for MatSegment {
    fn from(dot_vox_data: &DotVoxData) -> Self {
        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut vol = Dyna::filled(
                Vec3::new(model.size.x, model.size.y, model.size.z),
                MatCell::empty(),
                (),
            );

            for voxel in &model.voxels {
                let block = match voxel.i {
                    0 => MatCell::Mat(Material::Skin),
                    1 => MatCell::Mat(Material::Hair),
                    2 => MatCell::Mat(Material::EyeDark),
                    3 => MatCell::Mat(Material::EyeLight),
                    7 => MatCell::Mat(Material::EyeWhite),
                    //1 => MatCell::Mat(Material::HairLight),
                    //1 => MatCell::Mat(Material::HairDark),
                    //6 => MatCell::Mat(Material::Clothing),
                    index => {
                        let color = palette
                            .get(index as usize)
                            .copied()
                            .unwrap_or_else(|| Rgb::broadcast(0));
                        MatCell::Normal(color)
                    }
                };

                vol.set(
                    Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| i32::from(e)),
                    block,
                )
                .unwrap();
            }

            vol
        } else {
            Dyna::filled(Vec3::zero(), MatCell::empty(), ())
        }
    }
}
