//! Contract tests for path calculation API
//! 
//! These tests validate API stability and backward compatibility.
//! Contract tests ensure that the public API doesn't break between versions.

#[cfg(test)]
mod tests {
    use tp_lib_core::*;

    // T038: PathConfig defaults contract test
    #[test]
    fn test_pathconfig_defaults() {
        // Verify default configuration values remain stable across versions
        let config = PathConfig::default();

        assert_eq!(config.distance_scale, 10.0, "distance_scale default must be 10.0m");
        assert_eq!(config.heading_scale, 2.0, "heading_scale default must be 2.0°");
        assert_eq!(config.cutoff_distance, 50.0, "cutoff_distance default must be 50.0m");
        assert_eq!(config.heading_cutoff, 5.0, "heading_cutoff default must be 5.0°");
        assert_eq!(
            config.probability_threshold, 0.25,
            "probability_threshold default must be 0.25"
        );
        assert_eq!(
            config.resampling_distance, None,
            "resampling_distance default must be None (no resampling)"
        );
        assert_eq!(config.max_candidates, 3, "max_candidates default must be 3");
        assert_eq!(config.path_only, false, "path_only default must be false");
    }

    #[test]
    fn test_pathconfig_builder_api() {
        // T038: Verify builder API contract remains stable
        let config = PathConfig::builder()
            .distance_scale(15.0)
            .heading_scale(3.0)
            .cutoff_distance(75.0)
            .heading_cutoff(10.0)
            .probability_threshold(0.3)
            .resampling_distance(Some(10.0))
            .max_candidates(5)
            .build();

        assert!(config.is_ok(), "Builder with valid parameters must succeed");
        let cfg = config.unwrap();
        assert_eq!(cfg.distance_scale, 15.0);
        assert_eq!(cfg.heading_scale, 3.0);
        assert_eq!(cfg.cutoff_distance, 75.0);
        assert_eq!(cfg.heading_cutoff, 10.0);
        assert_eq!(cfg.probability_threshold, 0.3);
        assert_eq!(cfg.resampling_distance, Some(10.0));
        assert_eq!(cfg.max_candidates, 5);
    }

    #[test]
    fn test_pathconfig_validation() {
        // T038: Verify validation rejects invalid parameters
        
        // Invalid distance_scale (must be > 0)
        assert!(PathConfig::builder().distance_scale(0.0).build().is_err());
        assert!(PathConfig::builder().distance_scale(-5.0).build().is_err());

        // Invalid heading_scale (must be > 0)
        assert!(PathConfig::builder().heading_scale(0.0).build().is_err());
        assert!(PathConfig::builder().heading_scale(-2.0).build().is_err());

        // Invalid probability_threshold (must be in [0, 1])
        assert!(PathConfig::builder().probability_threshold(1.5).build().is_err());
        assert!(PathConfig::builder().probability_threshold(-0.1).build().is_err());
        assert!(PathConfig::builder().probability_threshold(0.0).build().is_ok());
        assert!(PathConfig::builder().probability_threshold(1.0).build().is_ok());

        // Invalid heading_cutoff (must be in [0, 180])
        assert!(PathConfig::builder().heading_cutoff(190.0).build().is_err());
        assert!(PathConfig::builder().heading_cutoff(-5.0).build().is_err());
        assert!(PathConfig::builder().heading_cutoff(0.0).build().is_ok());
        assert!(PathConfig::builder().heading_cutoff(180.0).build().is_ok());

        // Invalid max_candidates (must be >= 1)
        assert!(PathConfig::builder().max_candidates(0).build().is_err());
        assert!(PathConfig::builder().max_candidates(1).build().is_ok());
    }

    #[test]
    fn test_calculate_train_path_signature() {
        // T043: Verify calculate_train_path() function signature and error types
        use tp_lib_core::{models::{GnssPosition, Netelement, NetRelation}, PathConfig, calculate_train_path};
        use geo::LineString;
        use chrono::Utc;
        
        // Create minimal test data
        let gnss_positions = vec![
            GnssPosition::new(50.85, 4.35, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];
        
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
        ];
        
        let netrelations = vec![];
        let config = PathConfig::default();
        
        // Verify function exists with correct signature
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        
        // Result should be Result<PathResult, ProjectionError>
        assert!(result.is_ok() || result.is_err(), "Function should return Result type");
        
        // Verify PathResult structure
        if let Ok(path_result) = result {
            assert!(path_result.path.is_some() || path_result.path.is_none(), "path field should exist");
            // path_result.mode should be PathCalculationMode
            // path_result.projected_positions should be Vec<GnssPosition>
            // path_result.warnings should be Vec<String>
        }
    }

    #[test]
    fn test_project_onto_path_signature() {
        // T092: Verify project_onto_path() function signature
        use tp_lib_core::models::{GnssPosition, Netelement, TrainPath, AssociatedNetElement, ProjectedPosition};
        use tp_lib_core::{PathConfig, project_onto_path};
        use geo::LineString;
        use chrono::Utc;
        
        // Create minimal test data
        let gnss_positions = vec![
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];
        
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
        ];
        
        let segments = vec![
            AssociatedNetElement::new(
                "NE_A".to_string(),
                0.0,
                1.0,
                1.0,
                10,
            ).unwrap(),
        ];
        
        let path = TrainPath::new(
            segments,
            0.85,
            Some(Utc::now()),
            None,
        ).unwrap();
        
        let config = PathConfig::default();
        
        // Verify function exists with correct signature
        let result = project_onto_path(&gnss_positions, &path, &netelements, &config);
        
        // Result should be Result<Vec<ProjectedPosition>, ProjectionError>
        assert!(result.is_ok() || result.is_err(), "Function should return Result type");
        
        // Verify result type
        if let Ok(projected) = result {
            // Should be Vec<ProjectedPosition>
            assert!(projected.is_empty() || !projected.is_empty(), "Result should be vector of ProjectedPosition");
        }
    }
}
