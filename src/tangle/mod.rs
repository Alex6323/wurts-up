mod models;

use models::*;

pub use models::Payload;

use rand::Rng;

use std::cmp::{max, min};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::time::Instant;

use dashmap::{DashMap as HashMap, DashSet as HashSet};

const YTRSI_DELTA: u64 = 2; // C1
const OTRSI_DELTA: u64 = 7; // C2
const BELOW_MAX_DEPTH: u64 = 15; // M

static TANGLE: AtomicPtr<Tangle> = AtomicPtr::new(ptr::null_mut());
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    if !INITIALIZED.compare_and_swap(false, true, Ordering::Relaxed) {
        TANGLE.store(Box::into_raw(Tangle::default().into()), Ordering::Relaxed);
    } else {
        panic!("Tangle already initialized");
    }
}

pub fn tangle() -> &'static Tangle {
    let tangle = TANGLE.load(Ordering::Relaxed);
    if tangle.is_null() {
        panic!("Tangle cannot be null");
    } else {
        unsafe { &*tangle }
    }
}

#[derive(Default)]
pub struct Tangle {
    pub vertices: HashMap<Id, Vertex>,

    // missing parents; TODO: add confirmation info to it so, that it can be immediatedly set to confirmed if a milestone came in earlier
    pub missing: HashMap<Id, Children>,
    pub seps: HashMap<Id, MilestoneIndex>,

    // solid entry points and their corresponding milestone index; TODO: use `IndexId` type
    pub tips: HashSet<Id>,
    pub lmi: AtomicU64,  // MilestoneIndex
    pub lsmi: AtomicU64, //MilestoneIndex
}

impl Tangle {
    pub fn insert(&self, id: Id, payload: Payload, ma: Id, pa: Id) {
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
            self.missing.remove(&id).map(|(_, v)| v).unwrap()
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

        // Here we analyze the type of payload; it's either a (string) message, or a milestone
        // (with an associated index)
        let confirmation = match payload {
            Payload::Message(_) => None,
            Payload::Milestone(index) => {
                println!(
                    "[insert    ] Milestone arrived with id={}, index={}",
                    id, index
                );

                self.lmi.store(index, Ordering::Relaxed);

                println!("[insert    ] LMI now at {}", index);

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
    fn propagate_state(&self, root: &Id) {
        let now = Instant::now();
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
                if let Some(mut vertex) = self.vertices.get_mut(&id) {
                    vertex.solid = true;

                    match vertex.payload {
                        Payload::Milestone(index) => {
                            self.lsmi.store(index, Ordering::Relaxed);

                            println!("[prop_state] LSMI now at {}", index);
                        }
                        _ => (),
                    }

                    vertex.otrsi = Some(otrsi);
                    vertex.ytrsi = Some(ytrsi);

                    // println!(
                    //     "[prop_state] Propagated solid={}, OTRSI={}, YTRSI={} onto {}",
                    //     vertex.solid, otrsi.0, ytrsi.0, id
                    // );

                    // maybe we can propagate state even further
                    for child in vertex.children.iter() {
                        children.push(*child);
                    }
                }
            }
        }

        println!("[prop_state] Finished in {:?}", now.elapsed());
    }

    // TODO: barrier?

    // NOTE: this method confirms what it has in its past-cone whether it's solid or not
    fn confirm_recent_cone(&self, ma: &Id, pa: &Id, index: MilestoneIndex) -> Vec<Id> {
        let now = Instant::now();
        let mut visited = vec![*ma, *pa];
        let mut collected = Vec::new();

        while let Some(id) = visited.pop() {
            if let Some(mut vertex) = self.vertices.get_mut(&id) {
                if vertex.confirmation.is_none() {
                    // println!(
                    //     "[confirm   ] Confirmed vertex with id={} (ms_index={})",
                    //     id, index
                    // );

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

        println!("[confirm   ] Finished in {:?}", now.elapsed());
        collected
    }

    // NOTE: so once a milestone comes in we have to walk the future cones of the root transactions and update their
    // OTRSI and YTRSI
    fn update_snapshot_indices(&self, mut collected: Vec<Id>, index: MilestoneIndex) {
        let now = Instant::now();
        let mut children = Vec::new();
        let mut updated = std::collections::HashSet::new();

        while let Some(id) = collected.pop() {
            children.clear();

            // NOTE: Rust borrow rules force us to first create a children vec
            let (otrsi, ytrsi) = if let Some(vertex) = self.vertices.get(&id) {
                for child in vertex.children.iter() {
                    children.push(*child);
                }
                (vertex.otrsi.unwrap().0, vertex.ytrsi.unwrap().0)
            } else {
                panic!("[update rsi] Vertex not found");
            };

            for child in &children {
                if let Some(mut vertex2) = self.vertices.get_mut(&child) {
                    if vertex2.confirmation.is_some() {
                        // NOTE: we can ignore already confirmed vertices
                        // println!("[update rsi] No update required: {}", child);
                        continue;
                    }

                    if let Some(index_id) = vertex2.otrsi {
                        if index_id.1 == id {
                            // println!(
                            //     "[update rsi] Updating otrsi={} in {} from {}",
                            //     otrsi, child, id
                            // );

                            //index_id = IndexId(otrsi, id);
                            vertex2.otrsi.replace(IndexId(otrsi, id));
                        }
                    }

                    if let Some(index_id) = vertex2.ytrsi {
                        if index_id.1 == id {
                            // println!(
                            //     "[update rsi] Updating ytrsi={} in {} from {}",
                            //     ytrsi, child, id
                            // );

                            vertex2.ytrsi.replace(IndexId(ytrsi, id));
                        }
                    }

                    if !updated.contains(child) {
                        //println!("[update rsi] Proceding with {}", child);

                        collected.push(*child);
                    }
                }
            }

            updated.insert(id);
        }

        println!("[update rsi] Finished in {:?}", now.elapsed());
    }

    // Allows us to define certain `Id`s as solid entry points.
    pub fn add_solid_entrypoint(&self, id: Id, index: MilestoneIndex) {
        self.seps.insert(id, index);
    }

    pub fn is_solid(&self, id: &Id) -> bool {
        if let Some(vertex) = self.vertices.get(&id) {
            vertex.solid
        } else {
            self.is_sep(id) || self.check_db(id)
        }
    }

    pub fn is_sep(&self, id: &Id) -> bool {
        self.seps.contains_key(id)
    }

    pub fn is_milestone(&self, id: &Id) -> bool {
        if let Some(vertex) = self.vertices.get(&id) {
            vertex.payload.is_milestone()
        } else {
            false
        }
    }

    pub fn get_otrsi(&self, id: &Id) -> Option<MilestoneIndex> {
        if let Some(vertex) = self.vertices.get(&id) {
            vertex.otrsi.map(|index_id| index_id.0)
        } else {
            self.seps.get(id).map(|index| *index)
        }
    }

    pub fn get_ytrsi(&self, id: &Id) -> Option<MilestoneIndex> {
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

    pub fn confirmed(&self, id: &Id) -> Option<bool> {
        self.vertices
            .get(id)
            .map(|r| r.value().confirmation.is_some())
    }

    pub fn get(&self, id: &Id) -> Option<Payload> {
        self.vertices.get(id).map(|r| r.value().payload.clone())
    }

    pub fn num_tips(&self) -> usize {
        self.tips.len()
    }

    // TODO: add `select_two_tips` method
    pub fn select_two_tips(&self) -> Option<(Id, Id)> {
        if let Some(tip1) = self.select_tip() {
            if let Some(tip2) = self.select_tip() {
                return Some((tip1, tip2));
            }
        }

        None
    }

    /// Updates tip score, and performs the tip selection algorithm (TSA).
    pub fn select_tip(&self) -> Option<Id> {
        let now = Instant::now();

        // From all the tips create a subset "solid tips"
        let mut valid_tips = Vec::with_capacity(self.tips.len());
        let mut score_sum = 0_isize;
        let mut remove_list = Vec::new();

        if self.tips.is_empty() {
            return None;
        }

        for id in self.tips.iter() {
            if let Some(tip) = self.vertices.get(&id) {
                let otrsi = tip.otrsi.unwrap().0;
                let ytrsi = tip.ytrsi.unwrap().0;

                let score = self.get_tip_score(&id, otrsi, ytrsi) as isize;

                // NOTE: only non- and semi-lazy tips are considered for selection
                if !tip.solid || tip.selected > 2 || score == 0 {
                    remove_list.push(id);
                    continue;
                }

                //println!("added a valid with id={}, score={}", id, score);

                valid_tips.push((*id, score));
                score_sum += score;
            }
        }

        // TODO: remove invalid tips

        // TODO: randomly select tip
        let mut rng = rand::thread_rng();
        let mut random_number = rng.gen_range(1, score_sum);
        //println!("random_number={}", random_number);
        println!("[select_tip] tip pool size={}", valid_tips.len());

        for (id, score) in valid_tips.iter() {
            random_number -= score;
            if random_number <= 0 {
                if let Some(mut tip) = self.vertices.get_mut(id) {
                    tip.selected += 1;
                }
                println!("[select_tip] Selected tip={} in {:?}", id, now.elapsed());
                return Some(*id);
            }
        }

        println!("found no tip in {:?}", now.elapsed());
        None
    }

    #[inline]
    fn get_tip_score(&self, id: &Id, otrsi: MilestoneIndex, ytrsi: MilestoneIndex) -> Score {
        // NOTE: unwrap should be safe
        let vertex = self.vertices.get(&id).unwrap();

        if self.lsmi.load(Ordering::Relaxed) - ytrsi > YTRSI_DELTA {
            println!("[get_score ] YTRSI for {} too old", id);

            return Score::Lazy;
        }

        if self.lsmi.load(Ordering::Relaxed) - otrsi > BELOW_MAX_DEPTH {
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

            if self.lsmi.load(Ordering::Relaxed) - ma.otrsi.unwrap().0 > OTRSI_DELTA {
                parent_otrsi_check -= 1;
            }
        }

        if let Some(pa) = self.vertices.get(&pa) {
            // NOTE: removed as suggested by muxxer
            // if pa.score.unwrap_or(Score::NonLazy) == Score::Lazy {
            //     return Score::Lazy;
            // }

            if self.lsmi.load(Ordering::Relaxed) - pa.otrsi.unwrap().0 > OTRSI_DELTA {
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

    // TODO: remove this method eventually
    // For a given transaction finds all CRTs (confirmed root transactins).
    // NOTE: This method is not used during runtime. It's just to check that the OTRSI and YTRSI values are correctly propagated!
    // The first version of this prototype used it, and it was very very slow!
    pub fn scan_confirmed_root_transactions(&self, id: &Id) -> Option<(OTRSI, YTRSI)> {
        let mut visited = vec![*id];
        let mut collected = std::collections::HashSet::new();

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_milestone() {
        let tangle = make_tangle_1_milestone();

        for i in 1..=26 {
            assert!(tangle.is_solid(&i));
        }

        let confirmed = [1, 2, 3, 5, 6, 8, 9, 11, 12];

        for id in 1..26 {
            if confirmed.contains(&id) {
                assert!(tangle.confirmed(&id).unwrap());
            } else {
                assert!(!tangle.confirmed(&id).unwrap());
            }
        }

        assert_eq!(Some((1, 1)), tangle.scan_confirmed_root_transactions(&12));
        assert_eq!(Some((1, 1)), tangle.scan_confirmed_root_transactions(&22));
        // TODO: scan all tips to make sure propagation works as expected
    }

    #[test]
    fn two_milestones() {
        let tangle = make_tangle_2_milestones();

        for i in 1..=26 {
            assert!(tangle.is_solid(&i));
        }

        let confirmed = [1, 2, 3, 5, 6, 8, 9, 11, 15];

        for id in 1..26 {
            if confirmed.contains(&id) {
                assert!(tangle.confirmed(&id).unwrap());
            } else {
                assert!(!tangle.confirmed(&id).unwrap());
            }
        }

        assert_eq!(Some((1, 1)), tangle.scan_confirmed_root_transactions(&23));
        assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&24));
        assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&25));
        assert_eq!(Some((1, 2)), tangle.scan_confirmed_root_transactions(&26));
    }

    #[test]
    fn reversed_arrival() {
        let tangle = make_tangle_reversed_arrival();

        for i in 1..=26 {
            assert!(tangle.is_solid(&i));
        }

        let confirmed = [1, 2, 3, 5, 6, 8, 9, 11, 15];

        for id in 1..26 {
            if confirmed.contains(&id) {
                assert!(tangle.confirmed(&id).unwrap());
            } else {
                assert!(!tangle.confirmed(&id).unwrap());
            }
        }

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
}
