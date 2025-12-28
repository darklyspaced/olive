use std::{fmt::Display, rc::Rc};

use crate::green_node::{Green, GreenTree};

#[derive(Clone, Debug)]
/// Essentially a zipper data structure over a purely functional tree (the green tree). This is a
/// losless syntax tree.
pub struct SyntaxTree<'de>(Rc<RedData<'de>>);

#[derive(Debug)]
struct RedData<'de> {
    offset: usize,
    parent: Option<SyntaxTree<'de>>,
    green: Green<'de>,
}

impl<'de> SyntaxTree<'de> {
    pub fn new_root(root: GreenTree<'de>) -> Self {
        Self(Rc::new(RedData {
            parent: None,
            offset: 0,
            green: Green::Node(Rc::new(root)),
        }))
    }

    pub fn parent(&self) -> Option<Self> {
        self.0.parent.clone()
    }

    /// The red tree is essentially only built as we traverse so the general strategy here is to
    /// compute the absolute offsets of the children using the widths of the green nodes as a basis
    /// assuming that the root is 0 offset obviously.
    pub fn children(&'de self) -> impl Iterator<Item = SyntaxTree<'de>> {
        let mut offset = self.0.offset;
        let mut computed_children = vec![];

        for child in self.0.green.children() {
            let child_offset = offset;
            offset += child.width();
            let ptr = Rc::new(RedData {
                offset: child_offset,
                parent: Some(Self(Rc::clone(&self.0))),
                green: child.clone(),
            });
            computed_children.push(Self(Rc::clone(&ptr)));
        }

        computed_children.into_iter()
    }

    /// Just a helper function to print it out properly
    fn print(&self, indent: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let green = &self.0.green;
        let start = self.0.offset;
        let end = start + green.width();

        let pad = "  ".repeat(indent);

        match green {
            Green::Node(node) => {
                writeln!(f, "{}{:?}@{}..{}", pad, node.kind, start, end)?;
                for child in self.children() {
                    child.print(indent + 1, f)?;
                }
                Ok(())
            }
            Green::LeafT(tok) => {
                writeln!(f, "{}{:?}@{}..{}", pad, tok.kind, start, end)
            }
            Green::LeafN(_) => {
                writeln!(f, "{}Ident@{}..{}", pad, start, end)
            }
        }
    }
}

impl PartialEq for SyntaxTree<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.offset == other.0.offset && self.0.green == other.0.green
    }
}

impl Display for SyntaxTree<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.print(0, f)
    }
}
