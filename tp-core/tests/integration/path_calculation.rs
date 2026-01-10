//! Additional integration-style tests for complete path calculation workflow

use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
use tp_lib_core::{calculate_train_path, PathConfig};
use chrono::Utc;
use geo::LineString;

fn create_gnss(lat: f64, lon: f64, heading: Option<f64>) -> GnssPosition {
    let mut gnss = GnssPosition::new(lat, lon, Utc::now().into(), "EPSG:4326".to_string()).unwrap();
    gnss.heading = heading;
    gnss
}

fn create_netelement(id: &str, coords: Vec<(f64, f64)>) -> Netelement {
    Netelement::new(id.to_string(), LineString::from(coords), "EPSG:4326".to_string()).unwrap()
}

#[test]
fn test_calculate_path_single_netelement_multiple_positions() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![
        create_gnss(50.851, 4.351, None),
        create_gnss(50.853, 4.353, None),
        create_gnss(50.855, 4.355, None),
        create_gnss(50.857, 4.357, None),
    ];
    
    let netrelations = vec![];
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
    
    let path_result = result.unwrap();
    assert!(!path_result.projected_positions.is_empty());
}

#[test]
fn test_calculate_path_with_heading_constraints() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.850)]), // Eastward
        create_netelement("NE2", vec![(4.350, 50.850), (4.350, 50.860)]), // Northward
    ];
    
    // GNSS heading eastward (90 degrees)
    let gnss = vec![
        create_gnss(50.850, 4.351, Some(90.0)),
        create_gnss(50.850, 4.353, Some(90.0)),
    ];
    
    let netrelations = vec![];
    let config = PathConfig::builder()
        .heading_cutoff(45.0)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_path_branching_network() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.355, 50.855)]),
        create_netelement("NE2", vec![(4.355, 50.855), (4.360, 50.860)]),
        create_netelement("NE3", vec![(4.355, 50.855), (4.360, 50.850)]),
    ];
    
    let netrelations = vec![
        NetRelation::new("NR1".to_string(), "NE1".to_string(), "NE2".to_string(), 1, 0, true, false).unwrap(),
        NetRelation::new("NR2".to_string(), "NE1".to_string(), "NE3".to_string(), 1, 0, true, false).unwrap(),
    ];
    
    let gnss = vec![
        create_gnss(50.851, 4.351, None),
        create_gnss(50.856, 4.356, None),
    ];
    
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_path_circular_network() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.355, 50.850)]),
        create_netelement("NE2", vec![(4.355, 50.850), (4.355, 50.855)]),
        create_netelement("NE3", vec![(4.355, 50.855), (4.350, 50.855)]),
        create_netelement("NE4", vec![(4.350, 50.855), (4.350, 50.850)]),
    ];
    
    let netrelations = vec![
        NetRelation::new("NR1".to_string(), "NE1".to_string(), "NE2".to_string(), 1, 0, true, false).unwrap(),
        NetRelation::new("NR2".to_string(), "NE2".to_string(), "NE3".to_string(), 1, 0, true, false).unwrap(),
        NetRelation::new("NR3".to_string(), "NE3".to_string(), "NE4".to_string(), 1, 0, true, false).unwrap(),
        NetRelation::new("NR4".to_string(), "NE4".to_string(), "NE1".to_string(), 1, 0, true, false).unwrap(),
    ];
    
    let gnss = vec![
        create_gnss(50.850, 4.352, None),
        create_gnss(50.852, 4.355, None),
        create_gnss(50.855, 4.353, None),
    ];
    
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_path_max_candidates_limiting() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.351, 50.851)]),
        create_netelement("NE2", vec![(4.3501, 50.8501), (4.3511, 50.8511)]),
        create_netelement("NE3", vec![(4.3502, 50.8502), (4.3512, 50.8512)]),
        create_netelement("NE4", vec![(4.3503, 50.8503), (4.3513, 50.8513)]),
        create_netelement("NE5", vec![(4.3504, 50.8504), (4.3514, 50.8514)]),
    ];
    
    let gnss = vec![create_gnss(50.8502, 4.3502, None)];
    let netrelations = vec![];
    
    let config = PathConfig::builder()
        .max_candidates(2)
        .cutoff_distance(500.0)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_path_very_sparse_gnss() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.365, 50.865)]),
    ];
    
    // Only 2 GNSS points far apart
    let gnss = vec![
        create_gnss(50.851, 4.351, None),
        create_gnss(50.864, 4.364, None),
    ];
    
    let netrelations = vec![];
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_path_dense_gnss_with_resampling() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    // Create 20 closely spaced GNSS points
    let mut gnss = vec![];
    for i in 0..20 {
        let offset = i as f64 * 0.0005;
        gnss.push(create_gnss(50.850 + offset, 4.350 + offset, None));
    }
    
    let netrelations = vec![];
    let config = PathConfig::builder()
        .resampling_distance(Some(50.0))
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_path_unidirectional_netrelations() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.355, 50.855)]),
        create_netelement("NE2", vec![(4.355, 50.855), (4.360, 50.860)]),
    ];
    
    // NE1 -> NE2 is navigable forward only
    let netrelations = vec![
        NetRelation::new("NR1".to_string(), "NE1".to_string(), "NE2".to_string(), 1, 0, true, false).unwrap(),
    ];
    
    let gnss = vec![
        create_gnss(50.851, 4.351, None),
        create_gnss(50.856, 4.356, None),
    ];
    
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_path_all_modes_combined() {
    let netelements = vec![
        create_netelement("NE1", vec![(4.350, 50.850), (4.355, 50.855)]),
        create_netelement("NE2", vec![(4.355, 50.855), (4.360, 50.860)]),
    ];
    
    let netrelations = vec![
        NetRelation::new("NR1".to_string(), "NE1".to_string(), "NE2".to_string(), 1, 0, true, true).unwrap(),
    ];
    
    let gnss = vec![
        create_gnss(50.851, 4.351, Some(45.0)),
        create_gnss(50.856, 4.356, Some(45.0)),
    ];
    
    let config = PathConfig::builder()
        .distance_scale(12.0)
        .heading_scale(2.5)
        .cutoff_distance(60.0)
        .heading_cutoff(15.0)
        .probability_threshold(0.2)
        .max_candidates(4)
        .resampling_distance(Some(25.0))
        .path_only(false)
        .debug_mode(true)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
    
    let path_result = result.unwrap();
    assert!(path_result.debug_info.is_some());
}
