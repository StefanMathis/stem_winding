#[cfg(feature = "cairo")]
mod cairo_tests {

    use stem_winding::prelude::*;

    #[test]
    fn test_single_layer_18_4() {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 18.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::CoilSide,
        }
        .try_into()
        .unwrap();

        let mut layouter = EndWindingLayouter::new(&winding, true);
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(0, 0), Zone::new(2, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(3, 0), Zone::new(5, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(6, 0), Zone::new(8, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(9, 0), Zone::new(11, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(12, 0), Zone::new(14, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(15, 0), Zone::new(17, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(1, 0), Zone::new(4, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(7, 0), Zone::new(10, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(13, 0), Zone::new(16, 0)]
            );
        }
        assert!(layouter.next().is_none());
    }

    #[test]
    fn test_single_layer_18_1_dl() {
        let mut winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 18.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();
        winding.set_concentric_coils(true);

        let layouter = EndWindingLayouter::new(&winding, true);
        let coils_and_rank: Vec<_> = layouter.collect();
        assert_eq!(coils_and_rank.len(), 18);
    }

    #[test]
    fn test_single_layer_36_2() {
        let mut winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 36.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();
        winding.set_concentric_coils(true);

        let layouter = EndWindingLayouter::new(&winding, true);
        let coils_and_rank: Vec<_> = layouter.collect();
        assert_eq!(coils_and_rank.len(), 36);
    }

    #[test]
    fn test_single_layer_12_1_concentric() {
        let mut winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();
        winding.set_concentric_coils(true);

        let mut layouter = EndWindingLayouter::new(&winding, true);

        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(item.coil.phase().get(), 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(1, 0), Zone::new(6, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(item.coil.phase().get(), 3);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(9, 0), Zone::new(2, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 2);
            assert_eq!(item.coil.phase().get(), 2);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(5, 0), Zone::new(10, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 3);
            assert_eq!(item.coil.phase().get(), 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(0, 0), Zone::new(7, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 4);
            assert_eq!(item.coil.phase().get(), 3);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(8, 0), Zone::new(3, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 5);
            assert_eq!(item.coil.phase().get(), 2);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(4, 0), Zone::new(11, 0)]
            );
        }

        assert!(layouter.next().is_none());
    }
}
