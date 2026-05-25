# tp-lib Python Bindings

Python bindings for the Train Positioning Library (TP-Lib), providing GNSS track axis projection and train path calculation.

## Installation

```bash
pip install tp-lib
```

## Usage

```python
from tp_lib import project_positions, ProjectionConfig

config = ProjectionConfig(max_search_radius_meters=1000.0)
result = project_positions(gnss_positions, network, config)
```

### Automatic RINF Topology Retrieval

If you omit `network` (or pass `None`), the library downloads a bounding-box
subset of the ERA RINF topology on demand:

```python
from tp_lib import project_positions, RinfRetrievalOptions

result = project_positions(
    gnss_positions=gnss_positions,
    network=None,
    rinf_options=RinfRetrievalOptions(
        endpoint_url="https://graph.data.era.europa.eu/repositories/rinf-plus",
        buffer_meters=1000.0,
    ),
)
```

Typed errors (`InvalidGnssInputError`, `RinfMissingCoverageError`,
`RinfIncompleteTopologyError`, `RinfRetrievalFailedError`) are raised
when retrieval cannot satisfy the request. The same overload exists for
`calculate_train_path`.
