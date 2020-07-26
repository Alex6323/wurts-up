use std::cmp::Ordering;
use std::sync::atomic::AtomicU64;

use dashmap::DashSet as HashSet;

pub type Id = u64; // maybe the interned ternary hash
pub type MilestoneIndex = u64;
pub type AtomicMilestoneIndex = AtomicU64;
pub type OTRSI = MilestoneIndex;
pub type YTRSI = MilestoneIndex;
pub type Confirmation = Option<MilestoneIndex>;
pub type Children = HashSet<Id>;

#[derive(Clone, Debug)]
pub enum Transaction {
    Message(String),
    Milestone(MilestoneIndex),
}

impl Transaction {
    pub fn is_milestone(&self) -> bool {
        match *self {
            Self::Milestone(_) => true,
            _ => false,
        }
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::Message("".into())
    }
}

#[derive(Clone, Copy, Debug, Default, Ord, Eq)]
pub struct IndexId(pub MilestoneIndex, pub Id);

impl PartialOrd for IndexId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl PartialEq for IndexId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Default)]
pub struct Parents {
    pub ma: Id,
    pub pa: Id,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Score {
    Lazy = 0,
    SemiLazy = 1,
    NonLazy = 2,
}

#[derive(Default)]
pub struct Vertex {
    pub transaction: Transaction,
    pub parents: Parents,
    pub children: Children,
    pub solid: bool,
    pub valid: bool,
    pub confirmed: Confirmation,
    pub otrsi: Option<IndexId>, // can only be missing if ma and pa were missing; same for ytrsi
    pub ytrsi: Option<IndexId>,
    pub selected: u8, //number of times we selected it in the TSA
}
