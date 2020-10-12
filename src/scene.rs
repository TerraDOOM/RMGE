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
    pub fn new(node: SceneNode) -> SceneTree {
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

    pub fn unset_modifications(&mut self) {
        unset_modification(&mut self.root)
    }
}

fn unset_modification(node: &mut Tracked<SceneNode>) {
    for node in node.get_children_mut() {
        unset_modification(node)
    }

    println!(
        "{}",
        node.get_children()
            .map(|x| if x.is_modified() { "*" } else { "_" })
            .collect::<Vec<_>>()
            .join("")
    );
    node.reset();
}

#[derive(Debug)]
pub struct SceneNode {
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
        let cache = Tracked::new(trans.clone());
        let transform = Tracked::new(trans);

        SceneNode {
            cache,
            transform,
            child_count_changed: false,
            quad_count_changed: false,
            children: Vec::new(),
            quads: Vec::new(),
        }
    }

    pub fn add_child(&mut self, node: SceneNode) {
        self.child_count_changed = true;
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

    pub fn get_quads(&self) -> &[Tracked<Quad>] {}

    pub fn get_children(&self) -> &[Tracked<SceneNode>] {}

    pub fn get_quads_mut(&mut self) -> &mut [Tracked<Quad>] {}

    pub fn get_children_mut(&mut self) -> &mut [Tracked<SceneNode>] {}

    pub fn cache(&self) -> &Tracked<Mat4> {
        &self.cache
    }
}
