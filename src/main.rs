#![allow(dead_code, unused_variables)]

use rand; // 0.7.3

// use rand::{Rng, SeedableRng, XorShiftRng};
use rand::{rngs::ThreadRng, Rng};

use std::cmp::{max, min, Ordering};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

type Id = u64; // maybe the interned ternary hash
type MilestoneIndex = u64;
type OTRSI = MilestoneIndex;
type YTRSI = MilestoneIndex;

const YTRSI_DELTA: u64 = 2; // C1
const OTRSI_DELTA: u64 = 7; // C2
const BELOW_MAX_DEPTH: u64 = 15; // M

enum Payload {
    Message(&'static str),
    Milestone(MilestoneIndex),
}

impl Payload {
    fn is_milestone(&self) -> bool {
        match *self {
            Self::Milestone(_) => true,
            _ => false,
        }
    }
}

impl Default for Payload {
    fn default() -> Self {
        Self::Message("")
    }
}

type Confirmation = Option<MilestoneIndex>;

#[derive(Clone, Copy, Debug, Default, Ord, Eq)]
struct IndexId(pub MilestoneIndex, pub Id);

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

#[derive(Default)]
struct Parents {
    pub ma: Id,
    pub pa: Id,
}

type Children = HashSet<Id>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
enum Score {
    Lazy = 0,
    SemiLazy = 1,
    NonLazy = 2,
}

#[derive(Default)]
struct Vertex {
    pub payload: Payload,
    pub parents: Parents,
    pub children: Children,
    pub confirmation: Confirmation,
    pub solid: bool,
    pub otrsi: Option<IndexId>, // can only be missing if ma and pa were missing; same for ytrsi
    pub ytrsi: Option<IndexId>,
    pub selected: u8, //number of times we selected it in the TSA
}

#[derive(Default)]
struct Tangle {
    pub vertices: HashMap<Id, Vertex>,
    pub missing: HashMap<Id, Children>, // missing parents; TODO: add confirmation info to it so, that it can be immediatedly set to confirmed if a milestone came in earlier
    pub seps: HashMap<Id, MilestoneIndex>, // solid entry points and their corresponding milestone index; TODO: use `IndexId` type
    pub tips: HashSet<Id>,
    pub lmi: MilestoneIndex,
    pub lsmi: MilestoneIndex,
    rng: ThreadRng,
}

impl Tangle {
    fn insert(&mut self, id: Id, payload: Payload, ma: Id, pa: Id) {
        // NOTE: this is just here to get a rough idea about how long inserting is; uncomment this an the print statement
        // at the end of t his method.
        //let now = Instant::now();

        self.tips.remove(&ma);
        self.tips.remove(&pa);

        let children = if !self.missing.contains_key(&id) {
            // no children yet; so *could* be a valid tip elligible for selecting
            self.tips.insert(id);

            HashSet::new()
        } else {
            self.missing.remove(&id).unwrap()
        };

        // Here we check if parent-1 ("ma") exists; if it does then we update it with
        // the newly inserted vertex link
        if let Some(ma) = self.vertices.get_mut(&ma) {
            ma.children.insert(id);
        } else {
            if !self.seps.contains_key(&ma) && !self.check_db(&ma) {
                // the parent is missing, but when it arrives we want to exclude it from the tip set
                self.missing.entry(ma).or_insert(HashSet::new()).insert(id);
            }
        }

        // Here we check if parent-2 ("pa") exists; if it does then we update it with
        // the newly inserted vertex link
        if let Some(pa) = self.vertices.get_mut(&pa) {
            pa.children.insert(id);
        } else {
            if !self.seps.contains_key(&pa) && !self.check_db(&pa) {
                // the parent is missing, but when it arrives we want to exclude it from the tip set
                self.missing.entry(pa).or_insert(HashSet::new()).insert(id);
            }
        }

        // Here we analyze the type of payload; it's either a (string) message, or a milestone (with an associated index)
        let confirmation = match payload {
            Payload::Message(_) => None,
            Payload::Milestone(index) => {
                println!(
                    "[insert    ] Milestone arrived with id={}, index={}",
                    id, index
                );

                self.lmi = index;
                println!("[insert    ] LMI now at {}", self.lmi);

                // NOTE: how to deal with the situation, that a milestone might not be solid?
                let collected = self.confirm_recent_cone(&ma, &pa, index);

                self.update_snapshot_indices(collected, index);

                Some(index)
            }
        };

        // Now we create a `Vertex`, that holds the payload (Message or Milestone) ...
        let vertex = Vertex {
            payload,
            parents: Parents { ma, pa },
            children,
            confirmation,
            ..Vertex::default() // default: unsolid
        };

        // ... and insert it.
        self.vertices.insert(id, vertex);

        // Here we propagate the state (solid, YTRSI, OTRSI) to its children (future cone)
        // `solid`: a child is solid, if its parents are solid (ma & pa)
        // `otrsi`: the otrsi of the child is the minimum of the otrsi's of its parents (min(ma.otrsi, pa.otrsi))
        // `ytrsi`: the ytrsi of the child is the maximum of the ytrsi`s of its parents (max(ma.ytrsi, pa.ytrsi))
        self.propagate_state(&id);

        //println!("[insert    ] Inserted transaction in {:?}", now.elapsed());
    }

    // NOTE: there are 3 things being propagated/inherited: solid flag, otrsi, and ytrsi
    fn propagate_state(&mut self, root: &Id) {
        let mut children = vec![*root];

        while let Some(id) = children.pop() {
            // NOTE: if it's already solid then we don't need to propagate a state change
            if self.is_solid(&id) {
                unreachable!("found already solid vertex during propagation to future cone");
                // continue;
            }

            let parents = self
                .vertices
                .get(&id)
                .map(|vertex| (vertex.parents.ma, vertex.parents.pa));

            if let Some((ma, pa)) = parents {
                // NOTE: we might want to propagate OTRSI and YTRSI even if the parents aren't solid

                if !self.is_solid(&ma) || !self.is_solid(&pa) {
                    continue;
                }

                // NOTE: if the vertex is solid, then it **must** have parents with set otrsi and ytrsi, hence unwrap is safe
                let otrsi = min(
                    IndexId(self.get_otrsi(&ma).unwrap(), ma),
                    IndexId(self.get_otrsi(&pa).unwrap(), pa),
                );
                let ytrsi = max(
                    IndexId(self.get_ytrsi(&ma).unwrap(), ma),
                    IndexId(self.get_ytrsi(&pa).unwrap(), pa),
                );

                // NOTE: we now know that we can set it solid
                if let Some(vertex) = self.vertices.get_mut(&id) {
                    vertex.solid = true;

                    match vertex.payload {
                        Payload::Milestone(index) => {
                            self.lsmi = index;

                            println!("[prop_state] LSMI now at {}", self.lsmi);
                        }
                        _ => (),
                    }

                    vertex.otrsi = Some(otrsi);
                    vertex.ytrsi = Some(ytrsi);

                    println!(
                        "[prop_state] Propagated solid={}, OTRSI={}, YTRSI={} onto {}",
                        vertex.solid, otrsi.0, ytrsi.0, id
                    );

                    // maybe we can propagate state even further
                    for child in &vertex.children {
                        children.push(*child);
                    }
                }
            }
        }
    }

    // NOTE: this method confirms what it has in its past-cone whether it's solid or not
    fn confirm_recent_cone(&mut self, ma: &Id, pa: &Id, index: MilestoneIndex) -> Vec<Id> {
        let mut visited = vec![*ma, *pa];
        let mut collected = Vec::new();

        while let Some(id) = visited.pop() {
            if let Some(vertex) = self.vertices.get_mut(&id) {
                if vertex.confirmation.is_none() {
                    println!(
                        "[confirm   ] Confirmed vertex with id={} (ms_index={})",
                        id, index
                    );

                    vertex.confirmation = Some(index);

                    // NOTE: Setting otrsi and ytrsi for  confirmed vertices - I think - prevents some branching,
                    // if the tip directly attaches to it
                    // NOTE: the confirmed vertex now points to itself with its otrsi and ytrsi (as it has become a root transaction)
                    vertex.otrsi = Some(IndexId(index, id));
                    vertex.ytrsi = Some(IndexId(index, id));

                    // NOTE: we collect the newly confirmed vertices
                    collected.push(id);

                    // Continue confirming its parents (if those aren't confirmed yet)
                    visited.push(vertex.parents.ma);
                    visited.push(vertex.parents.pa);
                }
            } else {
                if !self.is_sep(&id) {
                    todo!("[confirm   ] missing vertex: {}", id);
                }
            }
        }

        collected
    }

    // NOTE: so once a milestone comes in we have to walk the future cones of the root transactions and update their OTRSI and YTRSI
    fn update_snapshot_indices(&mut self, mut collected: Vec<Id>, index: MilestoneIndex) {
        let mut children = Vec::new();
        let mut updated = HashSet::new();

        while let Some(id) = collected.pop() {
            children.clear();

            // NOTE: Rust borrow rules force us to first create a children vec
            let (otrsi, ytrsi) = if let Some(vertex) = self.vertices.get(&id) {
                for child in &vertex.children {
                    children.push(*child);
                }
                (vertex.otrsi.unwrap().0, vertex.ytrsi.unwrap().0)
            } else {
                panic!("[update rsi] Vertex not found");
            };

            for child in &children {
                if let Some(vertex2) = self.vertices.get_mut(child) {
                    if vertex2.confirmation.is_some() {
                        // NOTE: we can ignore already confirmed vertices
                        println!("[update rsi] No update required: {}", child);
                        continue;
                    }

                    if let Some(index_id) = vertex2.otrsi {
                        if index_id.1 == id {
                            println!(
                                "[update rsi] Updating otrsi={} in {} from {}",
                                otrsi, child, id
                            );

                            //index_id = IndexId(otrsi, id);
                            vertex2.otrsi.replace(IndexId(otrsi, id));
                        }
                    }

                    if let Some(index_id) = vertex2.ytrsi {
                        if index_id.1 == id {
                            println!(
                                "[update rsi] Updating ytrsi={} in {} from {}",
                                ytrsi, child, id
                            );

                            vertex2.ytrsi.replace(IndexId(ytrsi, id));
                        }
                    }

                    if !updated.contains(child) {
                        println!("[update rsi] Proceding with {}", child);

                        collected.push(*child);
                    }
                }
            }

            updated.insert(id);
        }

        //println!("finished updating indices");
    }

    // Allows us to define certain `Id`s as solid entry points.
    fn add_solid_entrypoint(&mut self, id: Id, index: MilestoneIndex) {
        self.seps.insert(id, index);
    }

    fn is_solid(&self, id: &Id) -> bool {
        if let Some(vertex) = self.vertices.get(&id) {
            vertex.solid
        } else {
            self.is_sep(id)
        }
    }

    fn is_sep(&self, id: &Id) -> bool {
        self.seps.contains_key(id)
    }

    fn is_milestone(&self, id: &Id) -> bool {
        if let Some(vertex) = self.vertices.get(&id) {
            vertex.payload.is_milestone()
        } else {
            false
        }
    }

    fn get_otrsi(&self, id: &Id) -> Option<MilestoneIndex> {
        if let Some(vertex) = self.vertices.get(&id) {
            vertex.otrsi.map(|index_id| index_id.0)
        } else {
            self.seps.get(id).map(|index| *index)
        }
    }

    fn get_ytrsi(&self, id: &Id) -> Option<MilestoneIndex> {
        if let Some(vertex) = self.vertices.get(&id) {
            vertex.ytrsi.map(|index_id| index_id.0)
        } else {
            self.seps.get(id).map(|index| *index)
        }
    }

    // Checks wether the id (hash?) is in the db
    fn check_db(&self, _id: &Id) -> bool {
        // NOTE: the Id type is not used in the db, but the (slower) transaction hash instead
        false
    }

    fn get(&self, id: &Id) -> Option<&Vertex> {
        self.vertices.get(id)
    }

    /// For a given transaction finds all CRTs (confirmed root transactins).
    // NOTE: This method is not used during runtime. It's just to check that the OTRSI and YTRSI values are correctly propagated!
    // The first version of this prototype used it, and it was very very slow!
    fn scan_confirmed_root_transactions(&self, id: &Id) -> Option<(OTRSI, YTRSI)> {
        let mut visited = vec![*id];
        let mut collected = HashSet::new();

        while let Some(id) = visited.pop() {
            if let Some(vertex) = self.vertices.get(&id) {
                if let Some(index) = vertex.confirmation {
                    collected.insert(index);
                } else {
                    visited.push(vertex.parents.ma);
                    visited.push(vertex.parents.pa);
                }
            }
        }

        if collected.is_empty() {
            // should not happen
            None
        } else {
            Some((
                *collected.iter().min().unwrap(),
                *collected.iter().max().unwrap(),
            ))
        }
    }

    fn num_tips(&self) -> usize {
        self.tips.len()
    }

    /// Updates tip score, and performs the tip selection algorithm (TSA).
    fn select_tip(&mut self) -> Option<&Id> {
        let now = Instant::now();

        // From all the tips create a subset "solid tips"
        let mut valid_tips = Vec::with_capacity(self.tips.len());
        let mut score_sum = 0_isize;

        if self.tips.is_empty() {
            return None;
        }

        for id in &self.tips {
            // let score = if let Some((otrsi, ytrsi)) = self.scan_confirmed_root_transactions(&id) {
            //     self.get_tip_score(&id, otrsi, ytrsi) as isize
            // } else {
            //     0 as isize
            // };

            if let Some(tip) = self.vertices.get(id) {
                let otrsi = tip.otrsi.unwrap().0;
                let ytrsi = tip.ytrsi.unwrap().0;

                let score = self.get_tip_score(&id, otrsi, ytrsi) as isize;

                // Unwrap should be save since all tips have scores
                //let score = tip.score.unwrap() as isize;

                //println!("id={}, solid={}, selected={}, score={}", id, tip.solid, tip.selected, score);

                // NOTE: only non- and semi-lazy tips are considered for selection
                if !tip.solid || tip.selected > 2 || score == 0 {
                    continue;
                }

                //println!("added a valid with id={}, score={}", id, score);

                valid_tips.push((id, score));
                score_sum += score;
            }
        }

        // TODO: randomly select tip
        let mut random_number = self.rng.gen_range(1, score_sum);
        //println!("random_number={}", random_number);

        for (id, score) in &valid_tips {
            random_number -= score;
            if random_number <= 0 {
                if let Some(tip) = self.vertices.get_mut(id) {
                    tip.selected += 1;
                }
                println!("[select_tip] Selected tip in {:?}", now.elapsed());
                return Some(id);
            }
        }

        println!("found no tip in {:?}", now.elapsed());
        None
    }

    fn get_tip_score(&self, id: &Id, otrsi: MilestoneIndex, ytrsi: MilestoneIndex) -> Score {
        // NOTE: unwrap should be safe
        let vertex = self.vertices.get(&id).unwrap();

        if self.lsmi - ytrsi > YTRSI_DELTA {
            println!("[get_score ] YTRSI for {} too old", id);

            return Score::Lazy;
        }

        if self.lsmi - otrsi > BELOW_MAX_DEPTH {
            println!("[get_score ] OTRSI for {} too old (below max depth)", id);

            return Score::Lazy;
        }

        let Parents { ma, pa } = vertex.parents;

        let mut parent_otrsi_check = 2;

        if let Some(ma) = self.vertices.get(&ma) {
            // NOTE: removed as suggested by muxxer
            // if ma.score.unwrap_or(Score::NonLazy) == Score::Lazy {
            //     return Score::Lazy;
            // }

            if self.lsmi - ma.otrsi.unwrap().0 > OTRSI_DELTA {
                parent_otrsi_check -= 1;
            }
        }

        if let Some(pa) = self.vertices.get(&pa) {
            // NOTE: removed as suggested by muxxer
            // if pa.score.unwrap_or(Score::NonLazy) == Score::Lazy {
            //     return Score::Lazy;
            // }

            if self.lsmi - pa.otrsi.unwrap().0 > OTRSI_DELTA {
                parent_otrsi_check -= 1;
            }
        }

        if parent_otrsi_check == 0 {
            println!("[get_score ] both parents failed 'parent_otrsi_check");

            return Score::Lazy;
        }

        if parent_otrsi_check == 1 {
            println!(
                "[get_score ] one of the parents failed 'parent_otrsi_check (makes tip semi-lazy)"
            );

            return Score::SemiLazy;
        }

        Score::NonLazy
    }
}

fn main() {
    // one_milestone();
    two_milestones();
    // four_tips();
    //reversed_arrival();
}

#[test]
fn one_milestone() {
    let mut tangle = make_tangle_1_milestone();

    assert!(tangle.get(&1).unwrap().solid);
    assert!(tangle.get(&2).unwrap().solid);
    assert!(tangle.get(&3).unwrap().solid);
    assert!(tangle.get(&4).unwrap().solid);
    assert!(tangle.get(&5).unwrap().solid);
    assert!(tangle.get(&6).unwrap().solid);
    assert!(tangle.get(&7).unwrap().solid);
    assert!(tangle.get(&8).unwrap().solid);
    assert!(tangle.get(&9).unwrap().solid);
    assert!(tangle.get(&10).unwrap().solid);
    assert!(tangle.get(&11).unwrap().solid);
    assert!(tangle.get(&12).unwrap().solid);
    assert!(tangle.get(&13).unwrap().solid);
    assert!(tangle.get(&14).unwrap().solid);
    assert!(tangle.get(&15).unwrap().solid);
    assert!(tangle.get(&16).unwrap().solid);
    assert!(tangle.get(&17).unwrap().solid);
    assert!(tangle.get(&18).unwrap().solid);
    assert!(tangle.get(&19).unwrap().solid);
    assert!(tangle.get(&20).unwrap().solid);
    assert!(tangle.get(&21).unwrap().solid);
    assert!(tangle.get(&22).unwrap().solid);
    assert!(tangle.get(&23).unwrap().solid);
    assert!(tangle.get(&24).unwrap().solid);
    assert!(tangle.get(&25).unwrap().solid);
    assert!(tangle.get(&26).unwrap().solid);

    assert!(tangle.get(&1).unwrap().confirmation.is_some());
    assert!(tangle.get(&2).unwrap().confirmation.is_some());
    assert!(tangle.get(&3).unwrap().confirmation.is_some());
    assert!(tangle.get(&4).unwrap().confirmation.is_none());
    assert!(tangle.get(&5).unwrap().confirmation.is_some());
    assert!(tangle.get(&6).unwrap().confirmation.is_some());
    assert!(tangle.get(&7).unwrap().confirmation.is_none());
    assert!(tangle.get(&8).unwrap().confirmation.is_some());
    assert!(tangle.get(&9).unwrap().confirmation.is_some());
    assert!(tangle.get(&10).unwrap().confirmation.is_none());
    assert!(tangle.get(&11).unwrap().confirmation.is_some());
    assert!(tangle.get(&12).unwrap().confirmation.is_some());
    assert!(tangle.get(&13).unwrap().confirmation.is_none());
    assert!(tangle.get(&14).unwrap().confirmation.is_none());
    assert!(tangle.get(&15).unwrap().confirmation.is_none());
    assert!(tangle.get(&16).unwrap().confirmation.is_none());
    assert!(tangle.get(&17).unwrap().confirmation.is_none());
    assert!(tangle.get(&18).unwrap().confirmation.is_none());
    assert!(tangle.get(&19).unwrap().confirmation.is_none());
    assert!(tangle.get(&20).unwrap().confirmation.is_none());
    assert!(tangle.get(&21).unwrap().confirmation.is_none());
    assert!(tangle.get(&22).unwrap().confirmation.is_none());
    assert!(tangle.get(&23).unwrap().confirmation.is_none());
    assert!(tangle.get(&24).unwrap().confirmation.is_none());
    assert!(tangle.get(&25).unwrap().confirmation.is_none());
    assert!(tangle.get(&26).unwrap().confirmation.is_none());

    assert_eq!(Some((1, 1)), tangle.scan_confirmed_root_transactions(&12));
    assert_eq!(Some((1, 1)), tangle.scan_confirmed_root_transactions(&22));

    println!("select_tip() = {}", tangle.select_tip().unwrap());
}

//#[test]
fn two_milestones() {
    let mut tangle = make_tangle_2_milestones();

    assert!(tangle.get(&1).unwrap().solid);
    assert!(tangle.get(&2).unwrap().solid);
    assert!(tangle.get(&3).unwrap().solid);
    assert!(tangle.get(&4).unwrap().solid);
    assert!(tangle.get(&5).unwrap().solid);
    assert!(tangle.get(&6).unwrap().solid);
    assert!(tangle.get(&7).unwrap().solid);
    assert!(tangle.get(&8).unwrap().solid);
    assert!(tangle.get(&9).unwrap().solid);
    assert!(tangle.get(&10).unwrap().solid);
    assert!(tangle.get(&11).unwrap().solid);
    assert!(tangle.get(&12).unwrap().solid);
    assert!(tangle.get(&13).unwrap().solid);
    assert!(tangle.get(&14).unwrap().solid);
    assert!(tangle.get(&15).unwrap().solid);
    assert!(tangle.get(&16).unwrap().solid);
    assert!(tangle.get(&17).unwrap().solid);
    assert!(tangle.get(&18).unwrap().solid);
    assert!(tangle.get(&19).unwrap().solid);
    assert!(tangle.get(&20).unwrap().solid);
    assert!(tangle.get(&21).unwrap().solid);
    assert!(tangle.get(&22).unwrap().solid);
    assert!(tangle.get(&23).unwrap().solid);
    assert!(tangle.get(&24).unwrap().solid);
    assert!(tangle.get(&25).unwrap().solid);
    assert!(tangle.get(&26).unwrap().solid);

    assert_eq!(1, tangle.get(&1).unwrap().confirmation.unwrap());
    assert_eq!(1, tangle.get(&2).unwrap().confirmation.unwrap());
    assert_eq!(1, tangle.get(&3).unwrap().confirmation.unwrap());
    assert!(tangle.get(&4).unwrap().confirmation.is_none());
    assert_eq!(1, tangle.get(&5).unwrap().confirmation.unwrap());
    assert_eq!(1, tangle.get(&6).unwrap().confirmation.unwrap());
    assert!(tangle.get(&7).unwrap().confirmation.is_none());
    assert_eq!(1, tangle.get(&8).unwrap().confirmation.unwrap());
    assert_eq!(2, tangle.get(&9).unwrap().confirmation.unwrap());
    assert!(tangle.get(&10).unwrap().confirmation.is_none());
    assert_eq!(2, tangle.get(&11).unwrap().confirmation.unwrap());
    assert!(tangle.get(&12).unwrap().confirmation.is_none());
    assert!(tangle.get(&13).unwrap().confirmation.is_none());
    assert!(tangle.get(&14).unwrap().confirmation.is_none());
    assert_eq!(2, tangle.get(&15).unwrap().confirmation.unwrap());
    assert!(tangle.get(&16).unwrap().confirmation.is_none());
    assert!(tangle.get(&17).unwrap().confirmation.is_none());
    assert!(tangle.get(&18).unwrap().confirmation.is_none());
    assert!(tangle.get(&19).unwrap().confirmation.is_none());
    assert!(tangle.get(&20).unwrap().confirmation.is_none());
    assert!(tangle.get(&21).unwrap().confirmation.is_none());
    assert!(tangle.get(&22).unwrap().confirmation.is_none());
    assert!(tangle.get(&23).unwrap().confirmation.is_none());
    assert!(tangle.get(&24).unwrap().confirmation.is_none());
    assert!(tangle.get(&25).unwrap().confirmation.is_none());
    assert!(tangle.get(&26).unwrap().confirmation.is_none());

    assert_eq!(Some((1, 1)), tangle.scan_confirmed_root_transactions(&23));
    assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&24));
    assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&25));
    assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&26));

    println!("select_tip() = {}", tangle.select_tip().unwrap());
}

#[test]
fn reversed_arrival() {
    let tangle = make_tangle_reversed_arrival();

    assert!(tangle.get(&1).unwrap().solid);
    assert!(tangle.get(&2).unwrap().solid);
    assert!(tangle.get(&3).unwrap().solid);
    assert!(tangle.get(&4).unwrap().solid);
    assert!(tangle.get(&5).unwrap().solid);
    assert!(tangle.get(&6).unwrap().solid);
    assert!(tangle.get(&7).unwrap().solid);
    assert!(tangle.get(&8).unwrap().solid);
    assert!(tangle.get(&9).unwrap().solid);
    assert!(tangle.get(&10).unwrap().solid);
    assert!(tangle.get(&11).unwrap().solid);
    assert!(tangle.get(&12).unwrap().solid);
    assert!(tangle.get(&13).unwrap().solid);
    assert!(tangle.get(&14).unwrap().solid);
    assert!(tangle.get(&15).unwrap().solid);
    assert!(tangle.get(&16).unwrap().solid);
    assert!(tangle.get(&17).unwrap().solid);
    assert!(tangle.get(&18).unwrap().solid);
    assert!(tangle.get(&19).unwrap().solid);
    assert!(tangle.get(&20).unwrap().solid);
    assert!(tangle.get(&21).unwrap().solid);
    assert!(tangle.get(&22).unwrap().solid);
    assert!(tangle.get(&23).unwrap().solid);
    assert!(tangle.get(&24).unwrap().solid);
    assert!(tangle.get(&25).unwrap().solid);
    assert!(tangle.get(&26).unwrap().solid);

    assert_eq!(1, tangle.get(&1).unwrap().confirmation.unwrap());
    assert_eq!(1, tangle.get(&2).unwrap().confirmation.unwrap());
    assert_eq!(1, tangle.get(&3).unwrap().confirmation.unwrap());
    assert!(tangle.get(&4).unwrap().confirmation.is_none());
    assert_eq!(1, tangle.get(&5).unwrap().confirmation.unwrap());
    assert_eq!(1, tangle.get(&6).unwrap().confirmation.unwrap());
    assert!(tangle.get(&7).unwrap().confirmation.is_none());
    assert_eq!(1, tangle.get(&8).unwrap().confirmation.unwrap());
    assert_eq!(2, tangle.get(&9).unwrap().confirmation.unwrap());
    assert!(tangle.get(&10).unwrap().confirmation.is_none());
    assert_eq!(2, tangle.get(&11).unwrap().confirmation.unwrap());
    assert!(tangle.get(&12).unwrap().confirmation.is_none());
    assert!(tangle.get(&13).unwrap().confirmation.is_none());
    assert!(tangle.get(&14).unwrap().confirmation.is_none());
    assert_eq!(2, tangle.get(&15).unwrap().confirmation.unwrap());
    assert!(tangle.get(&16).unwrap().confirmation.is_none());
    assert!(tangle.get(&17).unwrap().confirmation.is_none());
    assert!(tangle.get(&18).unwrap().confirmation.is_none());
    assert!(tangle.get(&19).unwrap().confirmation.is_none());
    assert!(tangle.get(&20).unwrap().confirmation.is_none());
    assert!(tangle.get(&21).unwrap().confirmation.is_none());
    assert!(tangle.get(&22).unwrap().confirmation.is_none());
    assert!(tangle.get(&23).unwrap().confirmation.is_none());
    assert!(tangle.get(&24).unwrap().confirmation.is_none());
    assert!(tangle.get(&25).unwrap().confirmation.is_none());
    assert!(tangle.get(&26).unwrap().confirmation.is_none());

    assert_eq!(0, tangle.missing.len());

    assert_eq!(Some((1, 1)), tangle.scan_confirmed_root_transactions(&23));
    assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&24));
    assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&25));
    assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&26));
}

#[test]
fn four_tips() {
    let tangle = make_tangle_1_milestone();

    assert_eq!(4, tangle.num_tips());
    assert_eq!(0, tangle.missing.len());
    assert_eq!(1, tangle.seps.len());
}

fn make_tangle_1_milestone() -> Tangle {
    let mut tangle = Tangle::default();

    tangle.add_solid_entrypoint(0, 0);

    tangle.insert(1, Payload::Message("A"), 0, 0);
    tangle.insert(2, Payload::Message("B"), 0, 0);
    tangle.insert(3, Payload::Message("C"), 0, 0);
    tangle.insert(4, Payload::Message("D"), 1, 2);
    tangle.insert(5, Payload::Message("E"), 1, 2);
    tangle.insert(6, Payload::Message("F"), 2, 3);
    tangle.insert(7, Payload::Message("G"), 4, 5);
    tangle.insert(8, Payload::Message("H"), 5, 6);
    tangle.insert(9, Payload::Message("I"), 6, 3);
    tangle.insert(10, Payload::Message("J"), 7, 8);
    tangle.insert(11, Payload::Message("K"), 8, 9);
    tangle.insert(12, Payload::Milestone(1), 8, 11); // MS 1
    tangle.insert(13, Payload::Message("L"), 7, 10);
    tangle.insert(14, Payload::Message("M"), 10, 8);
    tangle.insert(15, Payload::Message("N"), 11, 9);
    tangle.insert(16, Payload::Message("O"), 11, 9);
    tangle.insert(17, Payload::Message("P"), 13, 14);
    tangle.insert(18, Payload::Message("Q"), 13, 14);
    tangle.insert(19, Payload::Message("R"), 12, 15);
    tangle.insert(20, Payload::Message("S"), 15, 16);
    tangle.insert(21, Payload::Message("T"), 17, 18);
    tangle.insert(22, Payload::Message("U"), 18, 19);
    tangle.insert(23, Payload::Message("V"), 17, 21);
    tangle.insert(24, Payload::Message("W"), 21, 22);
    tangle.insert(25, Payload::Message("X"), 22, 18);
    tangle.insert(26, Payload::Message("Y"), 19, 20);

    tangle
}

fn make_tangle_2_milestones() -> Tangle {
    let mut tangle = Tangle::default();

    tangle.add_solid_entrypoint(0, 0);

    tangle.insert(1, Payload::Message("A"), 0, 0);
    tangle.insert(2, Payload::Message("B"), 0, 0);
    tangle.insert(3, Payload::Message("C"), 0, 0);
    tangle.insert(4, Payload::Message("D"), 1, 2);
    tangle.insert(5, Payload::Message("E"), 1, 2);
    tangle.insert(6, Payload::Message("F"), 2, 3);
    tangle.insert(7, Payload::Message("G"), 4, 5);
    tangle.insert(8, Payload::Milestone(1), 5, 6); // MS 1
    tangle.insert(9, Payload::Message("H"), 6, 3);
    tangle.insert(10, Payload::Message("I"), 7, 8);
    tangle.insert(11, Payload::Message("J"), 8, 9);
    tangle.insert(12, Payload::Message("K"), 8, 11);
    tangle.insert(13, Payload::Message("L"), 7, 10);
    tangle.insert(14, Payload::Message("M"), 10, 8);
    tangle.insert(15, Payload::Milestone(2), 11, 9); // MS 2
    tangle.insert(16, Payload::Message("N"), 11, 9);
    tangle.insert(17, Payload::Message("O"), 13, 14);
    tangle.insert(18, Payload::Message("P"), 13, 14);
    tangle.insert(19, Payload::Message("Q"), 12, 15);
    tangle.insert(20, Payload::Message("R"), 15, 16);
    tangle.insert(21, Payload::Message("S"), 17, 18);
    tangle.insert(22, Payload::Message("T"), 18, 19);
    tangle.insert(23, Payload::Message("U"), 17, 21);
    tangle.insert(24, Payload::Message("V"), 21, 22);
    tangle.insert(25, Payload::Message("W"), 22, 18);
    tangle.insert(26, Payload::Message("X"), 19, 20);

    tangle
}

fn make_tangle_reversed_arrival() -> Tangle {
    let mut tangle = Tangle::default();

    tangle.add_solid_entrypoint(0, 0);

    tangle.insert(1, Payload::Message("A"), 0, 0);
    tangle.insert(2, Payload::Message("B"), 0, 0);
    tangle.insert(3, Payload::Message("C"), 0, 0);
    tangle.insert(4, Payload::Message("D"), 1, 2);
    tangle.insert(5, Payload::Message("E"), 1, 2);
    tangle.insert(6, Payload::Message("F"), 2, 3);
    tangle.insert(7, Payload::Message("G"), 4, 5);
    tangle.insert(8, Payload::Milestone(1), 5, 6); // MS 1
    tangle.insert(9, Payload::Message("H"), 6, 3);
    tangle.insert(10, Payload::Message("I"), 7, 8);

    // reversed arrival
    tangle.insert(12, Payload::Message("K"), 8, 11);
    tangle.insert(11, Payload::Message("J"), 8, 9);

    tangle.insert(13, Payload::Message("L"), 7, 10);
    tangle.insert(14, Payload::Message("M"), 10, 8);
    tangle.insert(15, Payload::Milestone(2), 11, 9); // MS 2
    tangle.insert(16, Payload::Message("N"), 11, 9);
    tangle.insert(17, Payload::Message("O"), 13, 14);
    tangle.insert(18, Payload::Message("P"), 13, 14);
    tangle.insert(19, Payload::Message("Q"), 12, 15);
    tangle.insert(20, Payload::Message("R"), 15, 16);
    tangle.insert(21, Payload::Message("S"), 17, 18);
    tangle.insert(22, Payload::Message("T"), 18, 19);
    tangle.insert(23, Payload::Message("U"), 17, 21);
    tangle.insert(24, Payload::Message("V"), 21, 22);
    tangle.insert(25, Payload::Message("W"), 22, 18);
    tangle.insert(26, Payload::Message("X"), 19, 20);

    tangle
}
