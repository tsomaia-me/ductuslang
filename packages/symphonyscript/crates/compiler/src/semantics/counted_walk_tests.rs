use super::counted_walk::CountedWalk;

#[test]
fn every_walk_counts_the_step_it_takes() {
    for walk in CountedWalk::ALL {
        let before = walk.steps();
        walk.step();
        assert_eq!(walk.steps(), before + 1, "{walk:?}");
    }
}
