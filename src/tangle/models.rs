use std::cmp::Ordering;
use std::sync::atomic::AtomicU64;

use dashmap::DashSet as HashSet;

pub type InternedHash = u64;
pub type MilestoneIndex = u64;
pub type AtomicMilestoneIndex = AtomicU64;
pub type OTRSI = MilestoneIndex;
pub type YTRSI = MilestoneIndex;
pub type Confirmation = Option<MilestoneIndex>;
pub type Children = HashSet<InternedHash>;
pub type Payload = (); // this would be a: `bee-transaction::bundled::BundledTransaction`

#[derive(Clone, Copy, Debug, Default, Ord, Eq)]
pub struct IndexId(pub MilestoneIndex, pub InternedHash);

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
    pub ma: InternedHash,
    pub pa: InternedHash,
}

#[derive(Default)]
pub struct Message {
    pub payload: Payload,
    pub kind: MessageKind,
}

impl Message {
    pub fn new(payload: Payload, kind: MessageKind) -> Self {
        Self { payload, kind }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MessageKind {
    Data,
    Value,
    Checkpoint,
    Milestone(MilestoneIndex),
}

impl MessageKind {
    pub fn is_milestone(&self) -> bool {
        if let Self::Milestone(_) = *self {
            true
        } else {
            false
        }
    }
}

impl Default for MessageKind {
    fn default() -> Self {
        Self::Data
    }
}

#[derive(Copy, Clone, Default)]
pub struct Metadata {
    pub solid: bool,
    pub confirmed: Confirmation,
    pub otrsi: Option<IndexId>, // can only be missing if ma and pa were missing; same for ytrsi
    pub ytrsi: Option<IndexId>,
    pub selected: u8, //number of times we selected it in the TSA
}

#[derive(Default)]
pub struct Vertex {
    pub parents: Parents,
    pub children: Children,
    pub message: Message,
    pub metadata: Metadata,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Score {
    Lazy = 0,
    SemiLazy = 1,
    NonLazy = 2,
}
