use rmge::geometry::Mat4;
use rmge::scene::{SceneNode, SceneTree};

fn main() {
    let mut root = SceneNode::new(Mat4::identity());
    let mut child1 = SceneNode::new(2.0 * Mat4::identity());
    let mut child2 = SceneNode::new(3.0 * Mat4::identity());
    let child21 = SceneNode::new(Mat4::identity());
    let child22 = SceneNode::new(2.0 * Mat4::identity());

    child2.add_child(child21);
    child2.add_child(child22);

    root.add_child(child1);
    root.add_child(child2);

    let mut tree = SceneTree::new(root);

    display_tree(&tree);
    println!("--------------------------------------------------------------------------------");

    tree.recompute_caches();
    println!("--------------------------------------------------------------------------------");

    display_tree(&tree);

    println!("--------------------------------------------------------------------------------");
    println!("reset");
    tree.unset_modifications();
    println!("--------------------------------------------------------------------------------");

    display_tree(&tree);
}

fn display_tree(tree: &SceneTree) {
    let root = tree.root();

    println!("{}root:", if root.is_modified() { "*" } else { "" });
    println!(
        "{}cache: {:?}",
        star(&root.cache()),
        *root.transform.diagonal()
    );

    for (c1, cl1) in root.get_children().enumerate() {
        println!("\t{}c{}:", star(cl1), c1);
        println!(
            "\t{}cache: {:?}",
            star(&cl1.cache()),
            *cl1.transform.diagonal()
        );

        for (c2, cl2) in cl1.get_children().enumerate() {
            println!(
                "\t\t{}c{}{}, {}cache: {:?}",
                star(cl2),
                c1,
                c2,
                star(cl2.cache()),
                *cl2.cache().diagonal(),
            );
        }
    }
}

fn star<T: Unpin>(x: &rmge::tracker::Tracked<T>) -> &'static str {
    if x.is_modified() {
        "*"
    } else {
        ""
    }
}
