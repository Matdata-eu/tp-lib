# tp-lib Python Bindings

Python bindings for the Train Positioning Library (TP-Lib), providing GNSS track axis projection and train path calculation.

## Installation

```bash
pip install tp-lib
```

## Usage

### Timestamps

Input timestamps may be RFC3339 with an explicit offset (e.g.
`2025-12-09T14:30:00+01:00` or `2025-12-09T14:30:00Z`) or naive ISO 8601
(e.g. `2025-12-09T14:30:00`, `2025-12-09 14:30:00`). Naive values are
interpreted in the host's **local** timezone. All timestamps returned by
the library are RFC3339 strings carrying an explicit timezone offset.

```python
from tp_lib import project_gnss, ProjectionConfig

config = ProjectionConfig(max_search_radius_meters=1000.0)
result = project_gnss(gnss_positions, network, config)
```

### Automatic RINF Topology Retrieval

If you omit `network` (or pass `None`), the library downloads a bounding-box
subset of the ERA RINF topology on demand:

```python
from tp_lib import project_gnss, RinfRetrievalOptions

result = project_gnss(
    gnss_positions=gnss_positions,
    network=None,
    rinf_options=RinfRetrievalOptions(
        endpoint_url="https://graph.data.era.europa.eu/repositories/rinf-plus",
        buffer_meters=1000.0,
    ),
)
```

Errors are raised as built-in exceptions (`ValueError`, `RuntimeError`) when
retrieval cannot satisfy the request. The same overload exists for
`calculate_train_path`.

The library may raise:
- `ValueError` for invalid input (e.g., malformed GNSS data or invalid coordinates)
- `RuntimeError` for operational failures (e.g., RINF topology retrieval errors)
