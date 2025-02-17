// The node name representation was chosen to be in line with the notation used in the paper:
// In the paper 'w0' and 'w1' are the children of the node 'w', here we have 'self.1 << 1' and
// '(self.1 << 1) | 1'. This makes it easier to follow along.
use arrayvec::ArrayVec;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeName(u8, u32);

impl NodeName {
    pub const ROOT: Self = NodeName(0, 0);
    pub const MAX: Self = NodeName(32, u32::MAX);

    fn canonicalized(mut self) -> NodeName {
        let mut mask = u32::MAX;
        for i in self.0..32 {
            mask ^= 1 << i;
        }
        self.1 &= mask;
        self
    }

    pub fn new(length: u8, path: u32) -> Self {
        assert!(length <= 32);
        NodeName(length, path).canonicalized()
    }

    pub fn parent(self) -> NodeName {
        assert!(self.0 > 0);
        NodeName(self.0 - 1, self.1 >> 1)
    }

    pub fn left(self) -> NodeName {
        assert!(self.0 < 32);
        NodeName(self.0 + 1, self.1 << 1)
    }

    pub fn right(self) -> NodeName {
        assert!(self.0 < 32);
        NodeName(self.0 + 1, (self.1 << 1) | 1)
    }

    pub fn next(mut self) -> Option<NodeName> {
        if self == NodeName::MAX {
            None
        } else {
            if self.len() < 32 {
                self = self.left()
            } else {
                while self == self.parent().right() {
                    self = self.parent();
                }
                self = self.parent().right();
            }
            Some(self)
        }
    }

    pub fn len(self) -> u8 {
        self.0
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn path(self) -> u32 {
        self.1
    }

    pub fn is_leaf(self) -> bool {
        self.0 == 32
    }

    pub fn walk(mut self) -> impl Iterator<Item = NodeName> {
        let mut parents = ArrayVec::<NodeName, 32>::new();
        while !self.is_empty() {
            parents.push(self);
            self = self.parent();
        }
        parents.reverse();
        parents.into_iter()
    }

    pub fn from_numbering(mut number: u64) -> NodeName {
        // Number of nodes: 2**33 - 1
        assert!(number < 2u64.pow(33) - 1);
        let mut node = NodeName::ROOT;
        let mut cutoff = 2u64.pow(32);
        while cutoff > 0 {
            if number == 0 {
                return node;
            } else if number >= cutoff {
                node = node.right();
                number -= cutoff;
            } else {
                node = node.left();
                number -= 1;
            }
            cutoff /= 2;
        }
        node
    }

    pub fn to_numbering(self) -> u64 {
        let mut result = 0;
        let mut bonus = 2u64.pow(32);

        for node in self.walk() {
            let parent = node.parent();
            if node == parent.left() {
                result += 1;
            } else {
                result += bonus;
            }
            bonus /= 2;
        }

        result
    }

    pub fn in_subtree(self, other: NodeName) -> bool {
        other == Self::ROOT
            || (self.len() >= other.len() && self.walk().find(|n| *n == other).is_some())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn left_child() {
        assert_eq!(NodeName::ROOT.left(), NodeName::new(1, 0));
    }

    #[test]
    fn right_child() {
        assert_eq!(NodeName::ROOT.right(), NodeName::new(1, 1));
    }

    #[test]
    fn walk() {
        let node = NodeName::ROOT.left().right().right();
        let walk = node.walk().collect::<Vec<_>>();
        assert_eq!(
            walk,
            vec![
                NodeName::ROOT.left(),
                NodeName::ROOT.left().right(),
                NodeName::ROOT.left().right().right()
            ]
        );
    }

    #[test]
    fn from_numbering_root() {
        assert_eq!(NodeName::from_numbering(0), NodeName::ROOT);
    }

    #[test]
    fn from_numbering_node() {
        assert_eq!(
            NodeName::from_numbering(2u64.pow(32) + 1),
            NodeName::ROOT.right().left()
        );
    }

    #[test]
    fn to_numbering_root() {
        assert_eq!(NodeName::ROOT.to_numbering(), 0);
    }

    #[test]
    fn to_numbering_node() {
        assert_eq!(
            NodeName::ROOT.right().left().to_numbering(),
            2u64.pow(32) + 1,
        );
    }

    #[test]
    fn numbering_roundtrip() {
        let tests = [0u64, 1, 13, 42, 1337, 41238, 9182736, 1826455];
        for test in tests {
            assert_eq!(NodeName::from_numbering(test).to_numbering(), test);
        }
    }

    #[test]
    fn test_is_leaf() {
        let mut node = NodeName::ROOT;

        assert!(!node.is_leaf());

        for _ in 0..=30 {
            node = node.left();
            assert!(!node.is_leaf(), "{:?} is a leaf", node);
        }

        node = node.left();
        assert!(node.is_leaf());
    }

    #[test]
    fn test_next() {
        let mut node = NodeName::ROOT;

        for _ in 0..=31 {
            node = node.left();
        }

        node = node.next().unwrap();

        let mut correct = NodeName::ROOT;
        for _ in 0..=30 {
            correct = correct.left();
        }
        correct = correct.right();

        assert_eq!(node, correct);
    }

    #[test]
    fn test_in_subtree() {
        assert!(NodeName::ROOT.in_subtree(NodeName::ROOT));
        assert!(NodeName::ROOT.left().in_subtree(NodeName::ROOT.left()));
        assert!(NodeName::ROOT.left().in_subtree(NodeName::ROOT));
        assert!(NodeName::ROOT.right().in_subtree(NodeName::ROOT));
        assert!(NodeName::ROOT.left().right().in_subtree(NodeName::ROOT));
        assert!(NodeName::ROOT.left().left().in_subtree(NodeName::ROOT));
        assert!(!NodeName::ROOT.in_subtree(NodeName::ROOT.left()));
        assert!(!NodeName::ROOT.in_subtree(NodeName::ROOT.right()));
        assert!(!NodeName::ROOT.right().in_subtree(NodeName::ROOT.left()));
        assert!(!NodeName::ROOT.left().in_subtree(NodeName::ROOT.right()));
    }
}
