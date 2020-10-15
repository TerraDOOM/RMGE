use crate::geometry::{Mat4, Quad};
use crate::tracker::Tracked;

#[derive(Debug)]
pub struct SceneTree {
    root: Tracked<SceneNode>,
}

fn compute_cache(
    parent: &Tracked<Mat4>,
    mat: &Tracked<Mat4>,
    cache: &mut Tracked<Mat4>,
    children: &mut [Tracked<SceneNode>],
) {
    if parent.is_unmodified() {
        println!("parent unmodified");
        for child in children.iter_mut() {
            let new_parent = &*cache;
            if child.is_modified() {
                let &mut SceneNode {
                    ref transform,
                    ref mut cache,
                    children: ref mut new_children,
                    ..
                } = &mut **child;
                compute_cache(new_parent, transform, cache, new_children)
            }
        }
    } else {
        println!("parent modified");
        parent.mul_to(mat, &mut **cache);

        let new_parent = &*cache;
        for child in children.iter_mut() {
            let &mut SceneNode {
                ref transform,
                ref mut cache,
                children: ref mut new_children,
                ..
            } = &mut **child;
            compute_cache(new_parent, transform, cache, new_children);
        }
    }
}

impl SceneTree {
    pub fn new(mut node: SceneNode) -> SceneTree {
        node.df_index = Tracked::new(0);
        let mut root = Tracked::new(node);
        SceneTree { root }
    }

    pub fn root(&self) -> &Tracked<SceneNode> {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut Tracked<SceneNode> {
        &mut self.root
    }

    pub fn recompute_caches(&mut self) {
        if self.root.is_modified() {
            println!("root modified, recomputing caches");

            let &mut SceneNode {
                ref transform,
                ref mut cache,
                children: ref mut new_children,
                ..
            } = &mut *self.root;

            compute_cache(
                &Tracked::Modified(Mat4::identity()),
                transform,
                cache,
                new_children,
            );
        }
    }

    pub(crate) fn get_cache_and_quad_array(
        &mut self,
    ) -> (Vec<&mut Tracked<Mat4>>, Vec<Tracked<Quad>>) {
        unimplemented!()
    }

    pub fn unset_modifications(&mut self) {
        unset_modification(&mut self.root)
    }
}

fn unset_modification(node: &mut Tracked<SceneNode>) {
    for node in node.get_children_mut() {
        unset_modification(node)
    }
    node.reset();
}

#[derive(Debug)]
pub struct SceneNode {
    df_index: Tracked<usize>,
    // we try to maintain a cache of this "matrix multiplication chain"
    // i.e. the product M_1 * M_2 * ... * M_N where M_1 is the root and
    // M_N is either `transform` or the parent's `transform` (this part is as of
    //  yet undecided, as it doesn't change implementation much, and both have pros/cons).
    // Obviously, if any parent changes, the cache will be invalidated.
    pub(crate) cache: Tracked<Mat4>,
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
    pub fn new(trans: Mat4) -> Self {
        SceneNode {
            df_index: Tracked::new(0),
            cache: Tracked::new(trans),
            transform: Tracked::new(trans),
            child_count_changed: false,
            quad_count_changed: false,
            children: Vec::new(),
            quads: Vec::new(),
        }
    }

    pub fn add_child(&mut self, mut node: SceneNode) {
        self.child_count_changed = true;
        node.df_index = Tracked::new(self.children.len());
        assign_df_indices(*node.df_index, node.get_children_mut());
        let mut new = Tracked::new(node);
        self.children.push(new);
    }

    pub fn iter_quads(&self) -> impl Iterator<Item = &Tracked<Quad>> {
        self.quads.iter()
    }

    pub fn iter_children(&self) -> impl Iterator<Item = &Tracked<SceneNode>> {
        self.children.iter()
    }

    pub fn iter_quads_mut(&mut self) -> impl Iterator<Item = &mut Tracked<Quad>> {
        self.quads.iter_mut()
    }

    pub fn iter_children_mut(&mut self) -> impl Iterator<Item = &mut Tracked<SceneNode>> {
        self.children.iter_mut()
    }

    pub fn get_quads(&self) -> &[Tracked<Quad>] {
        &self.quads[..]
    }

    pub fn get_children(&self) -> &[Tracked<SceneNode>] {
        &self.children[..]
    }

    pub fn get_quads_mut(&mut self) -> &mut [Tracked<Quad>] {
        &mut self.quads[..]
    }

    pub fn get_children_mut(&mut self) -> &mut [Tracked<SceneNode>] {
        &mut self.children[..]
    }

    pub fn cache(&self) -> &Tracked<Mat4> {
        &self.cache
    }

    pub(crate) fn quads_df_index<'a>(
        &'a self,
    ) -> impl Iterator<Item = (usize, &'a Tracked<Quad>)> + 'a {
        let idx = *self.df_index;

        self.quads
            .iter()
            .map(move |tracked_quad| (idx, tracked_quad))
    }
}

fn assign_df_indices(parent_index: usize, nodes: &mut [Tracked<SceneNode>]) {
    for (c, node) in nodes.iter_mut().enumerate() {
        let cur_idx = parent_index + c;
        *((*node).df_index) = cur_idx;
        assign_df_indices(cur_idx, node.get_children_mut());
    }
}
