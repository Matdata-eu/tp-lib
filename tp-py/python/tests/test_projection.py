"""
Pytest tests for tp_lib Python bindings

Tests the Python API for GNSS projection functionality, including:
- Basic projection with valid inputs
- Error handling for invalid CRS
- Configuration parameter handling
- Data validation
"""

import pytest
import tempfile
import os
from pathlib import Path


# Only import if module is built (skip tests if not)
try:
    from tp_lib import project_gnss, ProjectionConfig, ProjectedPosition
    TP_LIB_AVAILABLE = True
except ImportError:
    TP_LIB_AVAILABLE = False


pytestmark = pytest.mark.skipif(not TP_LIB_AVAILABLE, reason="tp_lib extension not built")


# ============================================================================
# Test Fixtures
# ============================================================================

@pytest.fixture
def sample_gnss_csv(tmp_path):
    """Create a temporary GNSS CSV file"""
    csv_path = tmp_path / "positions.csv"
    csv_path.write_text(
        "latitude,longitude,timestamp\n"
        "50.8503,4.3517,2025-12-09T14:30:00+01:00\n"
        "50.8510,4.3525,2025-12-09T14:30:05+01:00\n"
        "50.8520,4.3530,2025-12-09T14:30:10+01:00\n"
    )
    return str(csv_path)


@pytest.fixture
def sample_network_geojson(tmp_path):
    """Create a temporary network GeoJSON file"""
    geojson_path = tmp_path / "network.geojson"
    geojson_path.write_text(
        """{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"id": "NE001"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.35, 50.85], [4.36, 50.86]]
      }
    },
    {
      "type": "Feature", 
      "properties": {"id": "NE002"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.36, 50.86], [4.37, 50.87]]
      }
    }
  ]
}"""
    )
    return str(geojson_path)


# ============================================================================
# Basic Projection Tests
# ============================================================================

def test_basic_projection(sample_gnss_csv, sample_network_geojson):
    """Test basic projection with default configuration"""
    results = project_gnss(
        gnss_file=sample_gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=sample_network_geojson,
        network_crs="EPSG:4326",
        target_crs="EPSG:31370"  # Belgian Lambert 72
    )
    
    # Should have same number of results as input positions
    assert len(results) == 3
    
    # Each result should be a ProjectedPosition
    for result in results:
        assert isinstance(result, ProjectedPosition)
        assert result.netelement_id in ["NE001", "NE002"]
        assert result.measure_meters >= 0
        assert result.projection_distance_meters >= 0
        assert result.crs == "EPSG:31370"
        assert -90 <= result.original_latitude <= 90
        assert -180 <= result.original_longitude <= 180


def test_projection_with_config(sample_gnss_csv, sample_network_geojson):
    """Test projection with custom configuration"""
    config = ProjectionConfig(
        max_search_radius_meters=500.0,
        projection_distance_warning_threshold=30.0,
        suppress_warnings=True
    )
    
    results = project_gnss(
        gnss_file=sample_gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=sample_network_geojson,
        network_crs="EPSG:4326",
        target_crs="EPSG:31370",
        config=config
    )
    
    assert len(results) == 3
    assert all(isinstance(r, ProjectedPosition) for r in results)


def test_projected_position_to_dict(sample_gnss_csv, sample_network_geojson):
    """Test ProjectedPosition.to_dict() method"""
    results = project_gnss(
        gnss_file=sample_gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=sample_network_geojson,
        network_crs="EPSG:4326",
        target_crs="EPSG:31370"
    )
    
    result_dict = results[0].to_dict()
    
    assert isinstance(result_dict, dict)
    assert "original_latitude" in result_dict
    assert "original_longitude" in result_dict
    assert "timestamp" in result_dict
    assert "projected_x" in result_dict
    assert "projected_y" in result_dict
    assert "netelement_id" in result_dict
    assert "measure_meters" in result_dict
    assert "projection_distance_meters" in result_dict
    assert "crs" in result_dict


def test_projected_position_repr(sample_gnss_csv, sample_network_geojson):
    """Test ProjectedPosition.__repr__() method"""
    results = project_gnss(
        gnss_file=sample_gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=sample_network_geojson,
        network_crs="EPSG:4326",
        target_crs="EPSG:31370"
    )
    
    repr_str = repr(results[0])
    assert "ProjectedPosition" in repr_str
    assert "netelement_id" in repr_str
    assert "measure" in repr_str


# ============================================================================
# Error Handling Tests
# ============================================================================

def test_invalid_gnss_crs(sample_gnss_csv, sample_network_geojson):
    """Test error handling for invalid GNSS CRS"""
    with pytest.raises(ValueError) as exc_info:
        project_gnss(
            gnss_file=sample_gnss_csv,
            gnss_crs="INVALID:9999",
            network_file=sample_network_geojson,
            network_crs="EPSG:4326",
            target_crs="EPSG:31370"
        )
    
    assert "Invalid CRS" in str(exc_info.value) or "crs" in str(exc_info.value).lower()


def test_invalid_network_crs(sample_gnss_csv, sample_network_geojson):
    """Test error handling for invalid network CRS"""
    with pytest.raises(ValueError) as exc_info:
        project_gnss(
            gnss_file=sample_gnss_csv,
            gnss_crs="EPSG:4326",
            network_file=sample_network_geojson,
            network_crs="INVALID:8888",
            target_crs="EPSG:31370"
        )
    
    assert "Invalid CRS" in str(exc_info.value) or "crs" in str(exc_info.value).lower()


def test_invalid_target_crs(sample_gnss_csv, sample_network_geojson):
    """Test error handling for invalid target CRS"""
    with pytest.raises(ValueError) as exc_info:
        project_gnss(
            gnss_file=sample_gnss_csv,
            gnss_crs="EPSG:4326",
            network_file=sample_network_geojson,
            network_crs="EPSG:4326",
            target_crs="INVALID:7777"
        )
    
    assert "Invalid CRS" in str(exc_info.value) or "crs" in str(exc_info.value).lower()


def test_missing_gnss_file(sample_network_geojson):
    """Test error handling for missing GNSS file"""
    with pytest.raises(IOError) as exc_info:
        project_gnss(
            gnss_file="/nonexistent/positions.csv",
            gnss_crs="EPSG:4326",
            network_file=sample_network_geojson,
            network_crs="EPSG:4326",
            target_crs="EPSG:31370"
        )
    
    assert "IO error" in str(exc_info.value) or "not found" in str(exc_info.value).lower()


def test_missing_network_file(sample_gnss_csv):
    """Test error handling for missing network file"""
    with pytest.raises(IOError) as exc_info:
        project_gnss(
            gnss_file=sample_gnss_csv,
            gnss_crs="EPSG:4326",
            network_file="/nonexistent/network.geojson",
            network_crs="EPSG:4326",
            target_crs="EPSG:31370"
        )
    
    assert "IO error" in str(exc_info.value) or "not found" in str(exc_info.value).lower()


def test_invalid_csv_format(tmp_path, sample_network_geojson):
    """Test error handling for invalid CSV format"""
    invalid_csv = tmp_path / "invalid.csv"
    invalid_csv.write_text("not,valid,csv\nformat,here,now\n")
    
    with pytest.raises((IOError, ValueError)) as exc_info:
        project_gnss(
            gnss_file=str(invalid_csv),
            gnss_crs="EPSG:4326",
            network_file=sample_network_geojson,
            network_crs="EPSG:4326",
            target_crs="EPSG:31370"
        )
    
    # Should raise some kind of error
    assert exc_info.value is not None


def test_invalid_geojson_format(tmp_path, sample_gnss_csv):
    """Test error handling for invalid GeoJSON format"""
    invalid_geojson = tmp_path / "invalid.geojson"
    invalid_geojson.write_text("{not valid json}")
    
    with pytest.raises((IOError, ValueError)) as exc_info:
        project_gnss(
            gnss_file=sample_gnss_csv,
            gnss_crs="EPSG:4326",
            network_file=str(invalid_geojson),
            network_crs="EPSG:4326",
            target_crs="EPSG:31370"
        )
    
    assert exc_info.value is not None


# ============================================================================
# Configuration Tests
# ============================================================================

def test_projection_config_defaults():
    """Test ProjectionConfig default values"""
    config = ProjectionConfig()
    
    assert config.max_search_radius_meters == 1000.0
    assert config.projection_distance_warning_threshold == 50.0
    assert config.suppress_warnings == False


def test_projection_config_custom():
    """Test ProjectionConfig with custom values"""
    config = ProjectionConfig(
        max_search_radius_meters=2000.0,
        projection_distance_warning_threshold=100.0,
        suppress_warnings=True
    )
    
    assert config.max_search_radius_meters == 2000.0
    assert config.projection_distance_warning_threshold == 100.0
    assert config.suppress_warnings == True


def test_projection_config_repr():
    """Test ProjectionConfig.__repr__() method"""
    config = ProjectionConfig(max_search_radius_meters=500.0)
    repr_str = repr(config)
    
    assert "ProjectionConfig" in repr_str
    assert "500" in repr_str


# ============================================================================
# Edge Cases
# ============================================================================

def test_empty_gnss_file(tmp_path, sample_network_geojson):
    """Test handling of empty GNSS CSV file"""
    empty_csv = tmp_path / "empty.csv"
    empty_csv.write_text("latitude,longitude,timestamp\n")
    
    results = project_gnss(
        gnss_file=str(empty_csv),
        gnss_crs="EPSG:4326",
        network_file=sample_network_geojson,
        network_crs="EPSG:4326",
        target_crs="EPSG:31370"
    )
    
    assert len(results) == 0


def test_single_position(tmp_path, sample_network_geojson):
    """Test projection with single GNSS position"""
    single_csv = tmp_path / "single.csv"
    single_csv.write_text(
        "latitude,longitude,timestamp\n"
        "50.8503,4.3517,2025-12-09T14:30:00+01:00\n"
    )
    
    results = project_gnss(
        gnss_file=str(single_csv),
        gnss_crs="EPSG:4326",
        network_file=sample_network_geojson,
        network_crs="EPSG:4326",
        target_crs="EPSG:31370"
    )
    
    assert len(results) == 1
    assert isinstance(results[0], ProjectedPosition)
