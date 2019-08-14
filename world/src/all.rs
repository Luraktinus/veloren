use serde_derive::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum ForestKind {
    Palm,
    Savannah,
    Oak,
    Pine,
    SnowPine,
}
