use crate::geometry::{Mat4, Quad};
use crate::tracker::Tracked;

pub struct SceneTree {
    root: SceneNode,
}

pub struct SceneNode {
    // we try to maintain a cache of this "matrix multiplication chain"
    // i.e. the product M_1 * M_2 * ... * M_N where M_1 is the root and
    // M_N is either `transform` or the parent's `transform` (this part is as of
    //  yet undecided, as it doesn't change implementation much, and both have pros/cons).
    // Obviously, if any parent changes, the cache will be invalidated.
    pub(crate) cache: Option<Mat4>,
    // the rightmost transform that will be applied to all the quads in this SceneNode
    pub transform: Tracked<Mat4>,
    // these two bools track whether elements were added or removed to/from either
    // of our two vectors. We track this separately since adding/removing anything
    // might trigger a reallocation of our gfx-hal buffers
    child_count_changed: bool,
    quad_count_changed: bool,
    children: Vec<Tracked<SceneNode>>,
    quads: Vec<Tracked<Quad>>,
}

impl SceneNode {
    pub fn get_quads(&mut self) -> impl Iterator<Item = &mut Tracked<Quad>> {
        self.quads.iter_mut()
    }

    pub fn get_children(&mut self) -> impl Iterator<Item = &mut Tracked<SceneNode>> {
        self.children.iter_mut()
    }
}
