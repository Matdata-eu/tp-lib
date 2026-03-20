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
