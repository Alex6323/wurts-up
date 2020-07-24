use crate::tangle::Payload;
use crate::tangle::{init, tangle};

// NOTE: this recreates the Tangle from the Protocol RFC 0008 (with 1 milestone)
pub fn make_tangle_1_milestone() -> (u64, u64) {
    init();

    tangle().add_solid_entrypoint(0, 0);

    tangle().insert(1, Payload::Message("A".into()), 0, 0);
    tangle().insert(2, Payload::Message("B".into()), 0, 0);
    tangle().insert(3, Payload::Message("C".into()), 0, 0);
    tangle().insert(4, Payload::Message("D".into()), 1, 2);
    tangle().insert(5, Payload::Message("E".into()), 1, 2);
    tangle().insert(6, Payload::Message("F".into()), 2, 3);
    tangle().insert(7, Payload::Message("G".into()), 4, 5);
    tangle().insert(8, Payload::Message("H".into()), 5, 6);
    tangle().insert(9, Payload::Message("I".into()), 6, 3);
    tangle().insert(10, Payload::Message("J".into()), 7, 8);
    tangle().insert(11, Payload::Message("K".into()), 8, 9);
    tangle().insert(12, Payload::Milestone(1), 8, 11); // MS 1
    tangle().insert(13, Payload::Message("L".into()), 7, 10);
    tangle().insert(14, Payload::Message("M".into()), 10, 8);
    tangle().insert(15, Payload::Message("N".into()), 11, 9);
    tangle().insert(16, Payload::Message("O".into()), 11, 9);
    tangle().insert(17, Payload::Message("P".into()), 13, 14);
    tangle().insert(18, Payload::Message("Q".into()), 13, 14);
    tangle().insert(19, Payload::Message("R".into()), 12, 15);
    tangle().insert(20, Payload::Message("S".into()), 15, 16);
    tangle().insert(21, Payload::Message("T".into()), 17, 18);
    tangle().insert(22, Payload::Message("U".into()), 18, 19);
    tangle().insert(23, Payload::Message("V".into()), 17, 21);
    tangle().insert(24, Payload::Message("W".into()), 21, 22);
    tangle().insert(25, Payload::Message("X".into()), 22, 18);
    tangle().insert(26, Payload::Message("Y".into()), 19, 20);

    (26, 1)
}

// NOTE: this recreates the Tangle from the Protocol RFC 0008 (with 2 milestones)
pub fn make_tangle_2_milestones() -> (u64, u64) {
    init();

    tangle().add_solid_entrypoint(0, 0);

    tangle().insert(1, Payload::Message("A".into()), 0, 0);
    tangle().insert(2, Payload::Message("B".into()), 0, 0);
    tangle().insert(3, Payload::Message("C".into()), 0, 0);
    tangle().insert(4, Payload::Message("D".into()), 1, 2);
    tangle().insert(5, Payload::Message("E".into()), 1, 2);
    tangle().insert(6, Payload::Message("F".into()), 2, 3);
    tangle().insert(7, Payload::Message("G".into()), 4, 5);
    tangle().insert(8, Payload::Milestone(1), 5, 6); // MS 1
    tangle().insert(9, Payload::Message("H".into()), 6, 3);
    tangle().insert(10, Payload::Message("I".into()), 7, 8);
    tangle().insert(11, Payload::Message("J".into()), 8, 9);
    tangle().insert(12, Payload::Message("K".into()), 8, 11);
    tangle().insert(13, Payload::Message("L".into()), 7, 10);
    tangle().insert(14, Payload::Message("M".into()), 10, 8);
    tangle().insert(15, Payload::Milestone(2), 11, 9); // MS 2
    tangle().insert(16, Payload::Message("N".into()), 11, 9);
    tangle().insert(17, Payload::Message("O".into()), 13, 14);
    tangle().insert(18, Payload::Message("P".into()), 13, 14);
    tangle().insert(19, Payload::Message("Q".into()), 12, 15);
    tangle().insert(20, Payload::Message("R".into()), 15, 16);
    tangle().insert(21, Payload::Message("S".into()), 17, 18);
    tangle().insert(22, Payload::Message("T".into()), 18, 19);
    tangle().insert(23, Payload::Message("U".into()), 17, 21);
    tangle().insert(24, Payload::Message("V".into()), 21, 22);
    tangle().insert(25, Payload::Message("W".into()), 22, 18);
    tangle().insert(26, Payload::Message("X".into()), 19, 20);

    (26, 2)
}

// pub fn make_tangle_reversed_arrival() -> Tangle {
//     let tangle = Tangle::default();

//     tangle.add_solid_entrypoint(0, 0);

//     tangle.insert(1, Payload::Message("A".into()), 0, 0);
//     tangle.insert(2, Payload::Message("B".into()), 0, 0);
//     tangle.insert(3, Payload::Message("C".into()), 0, 0);
//     tangle.insert(4, Payload::Message("D".into()), 1, 2);
//     tangle.insert(5, Payload::Message("E".into()), 1, 2);
//     tangle.insert(6, Payload::Message("F".into()), 2, 3);
//     tangle.insert(7, Payload::Message("G".into()), 4, 5);
//     tangle.insert(8, Payload::Milestone(1), 5, 6); // MS 1
//     tangle.insert(9, Payload::Message("H".into()), 6, 3);
//     tangle.insert(10, Payload::Message("I".into()), 7, 8);

//     // reversed arrival
//     tangle.insert(12, Payload::Message("K".into()), 8, 11);
//     tangle.insert(11, Payload::Message("J".into()), 8, 9);

//     tangle.insert(13, Payload::Message("L".into()), 7, 10);
//     tangle.insert(14, Payload::Message("M".into()), 10, 8);
//     tangle.insert(15, Payload::Milestone(2), 11, 9); // MS 2
//     tangle.insert(16, Payload::Message("N".into()), 11, 9);
//     tangle.insert(17, Payload::Message("O".into()), 13, 14);
//     tangle.insert(18, Payload::Message("P".into()), 13, 14);
//     tangle.insert(19, Payload::Message("Q".into()), 12, 15);
//     tangle.insert(20, Payload::Message("R".into()), 15, 16);
//     tangle.insert(21, Payload::Message("S".into()), 17, 18);
//     tangle.insert(22, Payload::Message("T".into()), 18, 19);
//     tangle.insert(23, Payload::Message("U".into()), 17, 21);
//     tangle.insert(24, Payload::Message("V".into()), 21, 22);
//     tangle.insert(25, Payload::Message("W".into()), 22, 18);
//     tangle.insert(26, Payload::Message("X".into()), 19, 20);

//     tangle
// }
