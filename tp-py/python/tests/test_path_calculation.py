"""
Pytest tests for tp_lib path-calculation and detections bindings.

Tests the Python API for:
- PathConfig construction & validation
- calculate_train_path on a tiny synthetic network
- PreparedDetections / prepare_detections workflow
- detection_provenance(), warnings, mode
"""

import json
import pytest
from pathlib import Path


try:
    from tp_lib import (
        calculate_train_path,
        PathConfig,
        PathResult,
        TrainPath,
        AssociatedNetElement,
        prepare_detections,
        PreparedDetections,
        ProjectedPosition,
    )
    TP_LIB_AVAILABLE = True
except ImportError:
    TP_LIB_AVAILABLE = False


pytestmark = pytest.mark.skipif(
    not TP_LIB_AVAILABLE, reason="tp_lib extension not built"
)


# ============================================================================
# Fixtures: a tiny two-segment network with GNSS trace running along it
# ============================================================================

# Coordinates along ~50.85N, 4.35E in Brussels area.
# Two adjacent LineStrings forming a single corridor:
#   NE001:  (4.350, 50.850) -> (4.355, 50.850)
#   NE002:  (4.355, 50.850) -> (4.360, 50.850)
# Connected end-to-end via a NetRelation.


@pytest.fixture
def network_geojson(tmp_path):
    path = tmp_path / "network.geojson"
    data = {
        "type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "EPSG:4326"}},
        "features": [
            {
                "type": "Feature",
                "properties": {"id": "NE001", "kind": "netelement"},
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[4.350, 50.850], [4.355, 50.850]],
                },
            },
            {
                "type": "Feature",
                "properties": {"id": "NE002", "kind": "netelement"},
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[4.355, 50.850], [4.360, 50.850]],
                },
            },
            {
                "type": "Feature",
                "properties": {
                    "id": "NR001",
                    "type": "netrelation",
                    "navigability": "both",
                    "netelementA": "NE001",
                    "positionOnA": 1,
                    "netelementB": "NE002",
                    "positionOnB": 0,
                },
                "geometry": None,
            },
        ],
    }
    path.write_text(json.dumps(data))
    return str(path)


@pytest.fixture
def gnss_csv(tmp_path):
    """Five GNSS samples along the corridor, 5s apart."""
    path = tmp_path / "gnss.csv"
    lines = ["latitude,longitude,timestamp"]
    # Move from x=4.3505 to 4.3595 in 5 steps
    timestamps = [
        "2025-12-09T14:30:00+01:00",
        "2025-12-09T14:30:05+01:00",
        "2025-12-09T14:30:10+01:00",
        "2025-12-09T14:30:15+01:00",
        "2025-12-09T14:30:20+01:00",
    ]
    xs = [4.3505, 4.3525, 4.3550, 4.3575, 4.3595]
    for x, ts in zip(xs, timestamps):
        lines.append(f"50.850000,{x},{ts}")
    path.write_text("\n".join(lines) + "\n")
    return str(path)


@pytest.fixture
def punctual_detection_csv(tmp_path):
    """Punctual detection anchored to NE001 at intrinsic 0.5, mid-trace."""
    path = tmp_path / "detections.csv"
    path.write_text(
        "id,timestamp,netelement_id,intrinsic\n"
        "D1,2025-12-09T14:30:10+01:00,NE001,0.5\n"
    )
    return str(path)


# ============================================================================
# PathConfig
# ============================================================================


def test_path_config_defaults():
    cfg = PathConfig()
    assert cfg.distance_scale == 10.0
    assert cfg.heading_scale == 2.0
    assert cfg.cutoff_distance == 500.0
    assert cfg.heading_cutoff == 10.0
    assert cfg.probability_threshold == 0.02
    assert cfg.max_candidates == 3
    assert cfg.path_only is False
    assert cfg.beta == 50.0
    assert cfg.edge_zone_distance == 50.0
    assert cfg.turn_scale == 30.0
    assert cfg.detection_cutoff_distance == 2.5
    assert cfg.resampling_distance is None


def test_path_config_custom():
    cfg = PathConfig(
        distance_scale=20.0,
        heading_scale=5.0,
        cutoff_distance=100.0,
        max_candidates=5,
        probability_threshold=0.05,
    )
    assert cfg.distance_scale == 20.0
    assert cfg.heading_scale == 5.0
    assert cfg.cutoff_distance == 100.0
    assert cfg.max_candidates == 5
    assert cfg.probability_threshold == 0.05


def test_path_config_repr():
    cfg = PathConfig()
    r = repr(cfg)
    assert "PathConfig" in r
    assert "distance_scale" in r


# ============================================================================
# calculate_train_path — happy path
# ============================================================================


def test_calculate_train_path_basic(gnss_csv, network_geojson):
    result = calculate_train_path(
        gnss_file=gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=network_geojson,
    )
    assert isinstance(result, PathResult)
    assert result.mode in ("topology_based", "fallback_independent")
    assert isinstance(result.warnings, list)
    assert isinstance(result.projected_positions, list)

    # Path should be found
    assert result.path is not None
    assert isinstance(result.path, TrainPath)
    assert 0.0 <= result.path.overall_probability <= 1.0

    # Segments
    segs = result.path.segments
    assert len(segs) >= 1
    for s in segs:
        assert isinstance(s, AssociatedNetElement)
        assert s.netelement_id in ("NE001", "NE002")
        assert 0.0 <= s.start_intrinsic <= 1.0
        assert 0.0 <= s.end_intrinsic <= 1.0
        assert 0.0 <= s.probability <= 1.0

    # Projected positions: count depends on internal filtering; we only
    # require that whatever is returned is shaped correctly.
    assert isinstance(result.projected_positions, list)
    for pos in result.projected_positions:
        assert isinstance(pos, ProjectedPosition)
        assert pos.netelement_id in ("NE001", "NE002")


def test_calculate_train_path_with_custom_config(gnss_csv, network_geojson):
    cfg = PathConfig(
        cutoff_distance=200.0,
        max_candidates=2,
        probability_threshold=0.01,
    )
    result = calculate_train_path(
        gnss_file=gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=network_geojson,
        config=cfg,
    )
    assert isinstance(result, PathResult)


def test_path_result_repr_and_provenance(gnss_csv, network_geojson):
    result = calculate_train_path(
        gnss_file=gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=network_geojson,
    )
    r = repr(result)
    assert "PathResult" in r
    # No detections supplied → empty provenance
    prov = result.detection_provenance()
    assert isinstance(prov, list)
    assert prov == []


# ============================================================================
# calculate_train_path — error cases
# ============================================================================


def test_calculate_train_path_missing_gnss(network_geojson):
    with pytest.raises(IOError):
        calculate_train_path(
            gnss_file="/nonexistent/gnss.csv",
            gnss_crs="EPSG:4326",
            network_file=network_geojson,
        )


def test_calculate_train_path_missing_network(gnss_csv):
    with pytest.raises(IOError):
        calculate_train_path(
            gnss_file=gnss_csv,
            gnss_crs="EPSG:4326",
            network_file="/nonexistent/network.geojson",
        )


def test_calculate_train_path_invalid_config(gnss_csv, network_geojson):
    """A negative max_candidates of 0 (or invalid value) should fail at build()."""
    cfg = PathConfig(probability_threshold=2.0)  # outside [0, 1]
    with pytest.raises((ValueError, RuntimeError)):
        calculate_train_path(
            gnss_file=gnss_csv,
            gnss_crs="EPSG:4326",
            network_file=network_geojson,
            config=cfg,
        )


# ============================================================================
# Detections
# ============================================================================


def test_prepare_detections_punctual(
    punctual_detection_csv, gnss_csv, network_geojson
):
    prepared = prepare_detections(
        detections_file=punctual_detection_csv,
        kind="punctual",
        gnss_file=gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=network_geojson,
    )
    assert isinstance(prepared, PreparedDetections)
    assert prepared.anchor_count >= 1
    assert isinstance(prepared.warnings, list)

    records = prepared.records()
    assert isinstance(records, list)
    assert len(records) >= 1
    rec = records[0]
    assert rec["kind"] == "punctual"
    assert rec["status"] in ("applied", "resolved", "discarded")
    assert "source_file" in rec
    assert "source_row" in rec
    assert "timestamp" in rec


def test_prepare_detections_invalid_kind(
    punctual_detection_csv, gnss_csv, network_geojson
):
    with pytest.raises(ValueError):
        prepare_detections(
            detections_file=punctual_detection_csv,
            kind="bogus",
            gnss_file=gnss_csv,
            gnss_crs="EPSG:4326",
            network_file=network_geojson,
        )


def test_calculate_train_path_with_detections(
    gnss_csv, network_geojson, punctual_detection_csv
):
    prepared = prepare_detections(
        detections_file=punctual_detection_csv,
        kind="punctual",
        gnss_file=gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=network_geojson,
    )

    result = calculate_train_path(
        gnss_file=gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=network_geojson,
        detections=prepared,
    )
    assert isinstance(result, PathResult)
    # Provenance is now populated from the supplied detections (shape only)
    prov = result.detection_provenance()
    assert isinstance(prov, list)


def test_prepared_detections_repr(
    punctual_detection_csv, gnss_csv, network_geojson
):
    prepared = prepare_detections(
        detections_file=punctual_detection_csv,
        kind="punctual",
        gnss_file=gnss_csv,
        gnss_crs="EPSG:4326",
        network_file=network_geojson,
    )
    r = repr(prepared)
    assert "PreparedDetections" in r
    assert "anchors=" in r
