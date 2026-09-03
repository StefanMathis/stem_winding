#[test]
fn test_parallel_paths() {
    let pp_iter: Vec<usize> = ParallelPathIterator::new(2).collect();
    assert_eq!(pp_iter, vec![1, 2]);

    let pp_iter: Vec<usize> = ParallelPathIterator::new(3).collect();
    assert_eq!(pp_iter, vec![1, 3]);

    let pp_iter: Vec<usize> = ParallelPathIterator::new(4).collect();
    assert_eq!(pp_iter, vec![1, 2, 4]);

    let pp_iter: Vec<usize> = ParallelPathIterator::new(8).collect();
    assert_eq!(pp_iter, vec![1, 2, 4, 8]);
}

#[test]
fn test_harmonic_ordinals() {
    {
        let winding =
            WindingToothCoil::new_minimal(24, 10, 3, 2, WindingTableMethod::Tingley).unwrap();

        let ordinals: Vec<Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], Ratio::new(-1, 5));
        assert_eq!(ordinals[1], Ratio::new(5, 5));
        assert_eq!(ordinals[2], Ratio::new(-7, 5));
        assert_eq!(ordinals[3], Ratio::new(11, 5));
        assert_eq!(ordinals[4], Ratio::new(-13, 5));
        assert_eq!(winding.harmonic_ordinals().coupling, -1);
    }

    {
        let winding =
            WindingToothCoil::new_minimal(24, 16, 3, 2, WindingTableMethod::Tingley).unwrap();
        let ordinals: Vec<Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], Ratio::new(-1, 2));
        assert_eq!(ordinals[1], Ratio::new(2, 2));
        assert_eq!(ordinals[2], Ratio::new(-4, 2));
        assert_eq!(ordinals[3], Ratio::new(5, 2));
        assert_eq!(ordinals[4], Ratio::new(-7, 2));
        assert_eq!(winding.harmonic_ordinals().coupling, -1);
    }

    {
        let winding =
            WindingToothCoil::new_minimal(9, 4, 3, 2, WindingTableMethod::Tingley).unwrap();
        let ordinals: Vec<Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], Ratio::new(1, 4));
        assert_eq!(ordinals[1], Ratio::new(-2, 4));
        assert_eq!(ordinals[2], Ratio::new(4, 4));
        assert_eq!(ordinals[3], Ratio::new(-5, 4));
        assert_eq!(ordinals[4], Ratio::new(7, 4));
        assert_eq!(winding.harmonic_ordinals().coupling, 1);
    }

    // From trait object
    {
        let winding_org =
            WindingToothCoil::new_minimal(9, 4, 3, 2, WindingTableMethod::Tingley).unwrap();
        let winding: &dyn Winding = &winding_org;
        let ordinals: Vec<Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], Ratio::new(1, 4));
        assert_eq!(ordinals[1], Ratio::new(-2, 4));
        assert_eq!(ordinals[2], Ratio::new(4, 4));
        assert_eq!(ordinals[3], Ratio::new(-5, 4));
        assert_eq!(ordinals[4], Ratio::new(7, 4));
        assert_eq!(winding.harmonic_ordinals().coupling, 1);
    }
}
