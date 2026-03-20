//! Unit tests for probability calculation module
//!
//! Tests for emission probability and Viterbi debug output.

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use geo::LineString;
    use tp_lib_core::models::{GnssPosition, Netelement};
    use tp_lib_core::{calculate_train_path, PathConfig};

    fn create_gnss(lat: f64, lon: f64) -> GnssPosition {
        GnssPosition::new(lat, lon, Utc::now().into(), "EPSG:4326".to_string()).unwrap()
    }

    // Test that debug netelement_probabilities are populated with
    // emission probabilities per candidate netelement.
    #[test]
    fn test_netelement_emission_probability_in_debug() {
        let netelements = vec![Netelement::new(
            "NE_LONG".to_string(),
            LineString::from(vec![(4.3400, 50.8400), (4.3530, 50.8530)]),
            "EPSG:4326".to_string(),
        )
        .unwrap()];

        let gnss = vec![
            create_gnss(50.8478, 4.3478),
            create_gnss(50.8490, 4.3490),
            create_gnss(50.8500, 4.3500),
            create_gnss(50.8510, 4.3510),
            create_gnss(50.8520, 4.3520),
            create_gnss(50.8528, 4.3528),
        ];

        let config = PathConfig::builder()
            .cutoff_distance(500.0)
            .probability_threshold(0.0)
            .debug_mode(true)
            .build()
            .unwrap();

        let result =
            calculate_train_path(&gnss, &netelements, &[], &config).expect("should succeed");

        let debug = result.debug_info.expect("debug info should be present");
        let ne_prob = debug
            .netelement_probabilities
            .iter()
            .find(|p| p.netelement_id == "NE_LONG")
            .expect("NE_LONG should be in probabilities");

        // The avg_emission_probability should be positive for close positions
        assert!(
            ne_prob.avg_emission_probability > 0.0,
            "avg_emission_probability should be positive, got {}",
            ne_prob.avg_emission_probability,
        );

        // Position count should equal number of GNSS positions that had
        // this netelement as a candidate
        assert!(
            ne_prob.position_count > 0,
            "position_count should be positive, got {}",
            ne_prob.position_count,
        );
    }

    // Test that Viterbi path membership is correctly indicated in debug output
    #[test]
    fn test_viterbi_path_membership_in_debug() {
        let netelements = vec![Netelement::new(
            "NE_800".to_string(),
            LineString::from(vec![(4.3400, 50.8400), (4.3480, 50.8480)]),
            "EPSG:4326".to_string(),
        )
        .unwrap()];

        let gnss = vec![
            create_gnss(50.8456, 4.3456),
            create_gnss(50.8462, 4.3462),
            create_gnss(50.8468, 4.3468),
            create_gnss(50.8474, 4.3474),
            create_gnss(50.8476, 4.3476),
        ];

        let config = PathConfig::builder()
            .cutoff_distance(500.0)
            .probability_threshold(0.0)
            .debug_mode(true)
            .build()
            .unwrap();

        let result =
            calculate_train_path(&gnss, &netelements, &[], &config).expect("should succeed");

        let debug = result.debug_info.expect("debug info");
        let ne_prob = debug
            .netelement_probabilities
            .iter()
            .find(|p| p.netelement_id == "NE_800")
            .expect("NE_800 should be in probabilities");

        // The netelement should have positive emission probability
        assert!(
            ne_prob.avg_emission_probability > 0.0,
            "avg_emission_probability should be positive, got {}",
            ne_prob.avg_emission_probability,
        );

        // With only one netelement, it should be in the Viterbi path
        assert!(
            ne_prob.in_viterbi_path,
            "NE_800 should be in the Viterbi path",
        );

        // It should not be a bridge (it was directly observed)
        assert!(!ne_prob.is_bridge, "NE_800 should not be a bridge",);
    }
}
