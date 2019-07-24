use super::{
    data::{
        LodData,
        LodConfig,
    },
    index::LodIndex,
    area::LodArea,
};

/*
    A LodDelta applies a change to a Lod
*/

pub trait LodDelta {
    type Config: LodConfig;

    fn apply(&self, data: &mut LodData::<Self::Config>);
    fn filter(&self, area: LodArea) -> Self;
}

pub struct DefaultLodDelta<X: LodConfig> {
    pub layer0: Vec<(LodIndex, Option<X::L0>)>, // 1/16
    pub layer1: Vec<(LodIndex, Option<X::L1>)>, // 1/8
    pub layer2: Vec<(LodIndex, Option<X::L2>)>, // 1/4
    pub layer3: Vec<(LodIndex, Option<X::L3>)>, // 1/2
    pub layer4: Vec<(LodIndex, Option<X::L4>)>, // 1
    pub layer5: Vec<(LodIndex, Option<X::L5>)>, // 2
    pub layer6: Vec<(LodIndex, Option<X::L6>)>, // 4
    pub layer7: Vec<(LodIndex, Option<X::L7>)>, // 8
    pub layer8: Vec<(LodIndex, Option<X::L8>)>, // 16
    pub layer9: Vec<(LodIndex, Option<X::L9>)>, // 32
    pub layer10: Vec<(LodIndex, Option<X::L10>)>, // 64
    pub layer11: Vec<(LodIndex, Option<X::L11>)>, // 128
    pub layer12: Vec<(LodIndex, Option<X::L12>)>, // 256
    pub layer13: Vec<(LodIndex, Option<X::L13>)>, // 512
    pub layer14: Vec<(LodIndex, Option<X::L14>)>, // 1024
    pub layer15: Vec<(LodIndex, Option<X::L15>)>,  // 2048
}

impl<X: LodConfig> DefaultLodDelta<X> {
    fn new() -> Self {
        Self {
            layer0: Vec::new(),
            layer1: Vec::new(),
            layer2: Vec::new(),
            layer3: Vec::new(),
            layer4: Vec::new(),
            layer5: Vec::new(),
            layer6: Vec::new(),
            layer7: Vec::new(),
            layer8: Vec::new(),
            layer9: Vec::new(),
            layer10: Vec::new(),
            layer11: Vec::new(),
            layer12: Vec::new(),
            layer13: Vec::new(),
            layer14: Vec::new(),
            layer15: Vec::new(),
        }
    }
}

impl<X: LodConfig> LodDelta for DefaultLodDelta<X> {
    type Config = X;

    fn apply(&self, data: &mut LodData::<Self::Config>) {
        // start with 15 -> 0 to create parents before childs
        // but thats not so good for deletions... mhhhh damit
        for (index, item) in &self.layer15 {
        }
    }

    fn filter(&self, area: LodArea) -> Self {
        Self::new()
    }
}