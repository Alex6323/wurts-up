use crate::tangle::{init, tangle};
use crate::tangle::{Message, MessageKind};

// NOTE: this recreates the Tangle from the Protocol RFC 0008 (with 1 milestone)
pub fn make_tangle_1_milestone() -> (u64, u64) {
    init();

    tangle().add_solid_entrypoint(0, 0);

    tangle().insert_gossip(1, Message::default(), 0, 0);
    tangle().insert_gossip(2, Message::default(), 0, 0);
    tangle().insert_gossip(3, Message::default(), 0, 0);
    tangle().insert_gossip(4, Message::default(), 1, 2);
    tangle().insert_gossip(5, Message::default(), 1, 2);
    tangle().insert_gossip(6, Message::default(), 2, 3);
    tangle().insert_gossip(7, Message::default(), 4, 5);
    tangle().insert_gossip(8, Message::default(), 5, 6);
    tangle().insert_gossip(9, Message::default(), 6, 3);
    tangle().insert_gossip(10, Message::default(), 7, 8);
    tangle().insert_gossip(11, Message::default(), 8, 9);
    tangle().insert_gossip(12, Message::new((), MessageKind::Milestone(1)), 8, 11); // MS 1
    tangle().insert_gossip(13, Message::default(), 7, 10);
    tangle().insert_gossip(14, Message::default(), 10, 8);
    tangle().insert_gossip(15, Message::default(), 11, 9);
    tangle().insert_gossip(16, Message::default(), 11, 9);
    tangle().insert_gossip(17, Message::default(), 13, 14);
    tangle().insert_gossip(18, Message::default(), 13, 14);
    tangle().insert_gossip(19, Message::default(), 12, 15);
    tangle().insert_gossip(20, Message::default(), 15, 16);
    tangle().insert_gossip(21, Message::default(), 17, 18);
    tangle().insert_gossip(22, Message::default(), 18, 19);
    tangle().insert_gossip(23, Message::default(), 17, 21);
    tangle().insert_gossip(24, Message::default(), 21, 22);
    tangle().insert_gossip(25, Message::default(), 22, 18);
    tangle().insert_gossip(26, Message::default(), 19, 20);

    (26, 1)
}

// NOTE: this recreates the Tangle from the Protocol RFC 0008 (with 2 milestones)
pub fn make_tangle_2_milestones() -> (u64, u64) {
    init();

    tangle().add_solid_entrypoint(0, 0);

    tangle().insert_gossip(1, Message::default(), 0, 0);
    tangle().insert_gossip(2, Message::default(), 0, 0);
    tangle().insert_gossip(3, Message::default(), 0, 0);
    tangle().insert_gossip(4, Message::default(), 1, 2);
    tangle().insert_gossip(5, Message::default(), 1, 2);
    tangle().insert_gossip(6, Message::default(), 2, 3);
    tangle().insert_gossip(7, Message::default(), 4, 5);
    tangle().insert_gossip(8, Message::new((), MessageKind::Milestone(1)), 5, 6); // MS 1
    tangle().insert_gossip(9, Message::default(), 6, 3);
    tangle().insert_gossip(10, Message::default(), 7, 8);
    tangle().insert_gossip(11, Message::default(), 8, 9);
    tangle().insert_gossip(12, Message::default(), 8, 11);
    tangle().insert_gossip(13, Message::default(), 7, 10);
    tangle().insert_gossip(14, Message::default(), 10, 8);
    tangle().insert_gossip(15, Message::new((), MessageKind::Milestone(2)), 11, 9); // MS 2
    tangle().insert_gossip(16, Message::default(), 11, 9);
    tangle().insert_gossip(17, Message::default(), 13, 14);
    tangle().insert_gossip(18, Message::default(), 13, 14);
    tangle().insert_gossip(19, Message::default(), 12, 15);
    tangle().insert_gossip(20, Message::default(), 15, 16);
    tangle().insert_gossip(21, Message::default(), 17, 18);
    tangle().insert_gossip(22, Message::default(), 18, 19);
    tangle().insert_gossip(23, Message::default(), 17, 21);
    tangle().insert_gossip(24, Message::default(), 21, 22);
    tangle().insert_gossip(25, Message::default(), 22, 18);
    tangle().insert_gossip(26, Message::default(), 19, 20);

    (26, 2)
}

// pub fn make_tangle_reversed_arrival() -> Tangle {
//     let tangle = Tangle::default();

//     tangle.add_solid_entrypoint(0, 0);

//     tangle.insert(1, Transaction::Message("A".into()), 0, 0);
//     tangle.insert(2, Transaction::Message("B".into()), 0, 0);
//     tangle.insert(3, Transaction::Message("C".into()), 0, 0);
//     tangle.insert(4, Transaction::Message("D".into()), 1, 2);
//     tangle.insert(5, Transaction::Message("E".into()), 1, 2);
//     tangle.insert(6, Transaction::Message("F".into()), 2, 3);
//     tangle.insert(7, Transaction::Message("G".into()), 4, 5);
//     tangle.insert(8, Transaction::Milestone(1), 5, 6); // MS 1
//     tangle.insert(9, Transaction::Message("H".into()), 6, 3);
//     tangle.insert(10, Transaction::Message("I".into()), 7, 8);

//     // reversed arrival
//     tangle.insert(12, Transaction::Message("K".into()), 8, 11);
//     tangle.insert(11, Transaction::Message("J".into()), 8, 9);

//     tangle.insert(13, Transaction::Message("L".into()), 7, 10);
//     tangle.insert(14, Transaction::Message("M".into()), 10, 8);
//     tangle.insert(15, Transaction::Milestone(2), 11, 9); // MS 2
//     tangle.insert(16, Transaction::Message("N".into()), 11, 9);
//     tangle.insert(17, Transaction::Message("O".into()), 13, 14);
//     tangle.insert(18, Transaction::Message("P".into()), 13, 14);
//     tangle.insert(19, Transaction::Message("Q".into()), 12, 15);
//     tangle.insert(20, Transaction::Message("R".into()), 15, 16);
//     tangle.insert(21, Transaction::Message("S".into()), 17, 18);
//     tangle.insert(22, Transaction::Message("T".into()), 18, 19);
//     tangle.insert(23, Transaction::Message("U".into()), 17, 21);
//     tangle.insert(24, Transaction::Message("V".into()), 21, 22);
//     tangle.insert(25, Transaction::Message("W".into()), 22, 18);
//     tangle.insert(26, Transaction::Message("X".into()), 19, 20);

//     tangle
// }
