# Test data: manually curated data and real train positions

Each log has its own subdirectory (`log_XXXXX/`) containing the source GNSS CSV and the three computed GeoJSON outputs.

- [Test data: manually curated data and real train positions](#test-data-manually-curated-data-and-real-train-positions)
  - [The network file](#the-network-file)
  - [Sample data](#sample-data)
  - [CLI quick reference](#cli-quick-reference)
  - [Easy cases (no track switches)](#easy-cases-no-track-switches)
    - [L36 track B – log\_28876](#l36-track-b--log_28876)
      - [The GNSS data](#the-gnss-data)
      - [Simple projection](#simple-projection)
      - [Path calculation](#path-calculation)
      - [Path projection](#path-projection)
    - [L36 track A – log\_29083](#l36-track-a--log_29083)
      - [The GNSS data](#the-gnss-data-1)
      - [Simple projection](#simple-projection-1)
      - [Path calculation](#path-calculation-1)
      - [Path projection](#path-projection-1)
  - [Single-switch cases](#single-switch-cases)
    - [L36-A → L36C-A – log\_28554](#l36-a--l36c-a--log_28554)
      - [Simple projection](#simple-projection-2)
      - [Path calculation](#path-calculation-2)
      - [Path projection](#path-projection-2)

Root folder for release exe files: target/release/

## The network file

The file `network_airport.geojson` contains a subpart of the Belgian railway network around the airport of Brussels/Zaventem. The tracks going to the airport are underground. At the same time there are tracks above ground but sometimes overcrossed by roads. 

Here's a GIS visualisation containing openstreetmap

![Network with openstreetmap background](static/network-osm.png)

In order to better see the tracks, find below a visualisation without background. Note: the red dots are the netrelations (switches):

![Network without openstreetmap background](static/network-no-osm.png)

## Sample data

The files `sample_gnss.geojson` and `sample_network.geojson` only serve for educational reasons.

## CLI quick reference

The three operations available via `tp-cli` are:

| Operation | Command | Description |
|-----------|---------|-------------|
| Simple projection | `tp-cli simple-projection ...` | Projects each GNSS point onto the nearest netelement (legacy) |
| Path calculation | `tp-cli calculate-path ...` | Calculates the most likely sequence of netelements without projecting |
| Path projection | `tp-cli ...` (no subcommand) | Full pipeline: calculates path then projects GNSS positions onto it |

Common flags: `--gnss <FILE>`, `--network <FILE>`, `--crs EPSG:4326`, `--output <FILE>` (`.geojson` extension auto-selects GeoJSON format).

## Easy cases (no track switches)

### L36 track B – log_28876

Log file ID: 28876

#### The GNSS data

Relatively clean GNSS data, train traveling from Leuven to Brussels on line 36, track B. The GNSS positions (green) are slightly offset to the north:

![L36 track B - Raw GNSS](log_28876/log_28876_L36-B-raw.png)

#### Simple projection

Tests to demonstrate how the simple projection works and where it fails (look at North going switches).

```bash
target/release/tp-cli.exe simple-projection --gnss test-data/log_28876/log_28876_L36-B.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_28876/log_28876_L36-B-simple-projection.geojson
```

The result is good, all GNSS positions are projected on the closest netelement. Note that this yields the expected outcome of GNSS projections on connecting tracks (red rectangles) and jumping back to the main track.

![L36 track B - Simple projection](log_28876/log_28876_L36-B-simple-projection.png)

#### Path calculation

Output should be a simple concatenation of all the netelements with a high probability. 

```bash
target/release/tp-cli.exe calculate-path --gnss test-data/log_28876/log_28876_L36-B.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_28876/log_28876_L36-B-path-calculation.geojson
```

Output is correct:
1. 88_L_3842  (prob=0.897)
2. 88_L_5900  (prob=0.855)
3. 88_L_11648 (prob=0.766)
4. 88_L_127   (prob=0.027)
5. 88_L_9748  (prob=0.733)

![L36 track B - Path calculation](log_28876/log_28876_L36-B-path.png)

Note the very low probability for 88_L_127. This is due to it being a very short netelement that connects two switches and only has a very limited number of GPS coordinates on it:

![L36 track B - Path calculation zoom](log_28876/log_28876_L36-B-path-zoom-link.png)

We might need to revise the algorithm later because of this.

#### Path projection

Finally we can evaluate the performance of projecting the coordinates onto the calculated path:

```bash
target/release/tp-cli.exe --gnss test-data/log_28876/log_28876_L36-B.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_28876/log_28876_L36-B-path-projection.geojson
```

![L36 track B - Path projection](log_28876/log_28876_L36-B-path-projection.png)

In gold the path projected coordinates. In blue the result of the simple projection as reference. We conclude that path projection yields better results than simple projection.

### L36 track A – log_29083

Log file ID: 29083

#### The GNSS data

Relatively dirty GNSS data, train traveling from Brussels to Leuven on line 36, track A. The GNSS positions (green) jump about 200m away from the track center:

![L36 track A - Raw GNSS](log_29083/log_29083_L36-A-raw.png)

#### Simple projection

```bash
target/release/tp-cli.exe simple-projection --gnss test-data/log_29083/log_29083_L36-A.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_29083/log_29083_L36-A-simple-projection.geojson
```

We see exactly what we would predict, the simple projection algorithm will think the train is going towards the airport station:

![L36 track A - Simple projection](log_29083/log_29083_L36-A-simple-projection.png)

#### Path calculation

```bash
target/release/tp-cli.exe calculate-path --gnss test-data/log_29083/log_29083_L36-A.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_29083/log_29083_L36-A-path-calculation.geojson
```

Output is correct, no surprises:
1. 88_L_5916 (prob=0.629)
2. 88_L_2026  (prob=1.000)
3. 88_L_42    (prob=0.667)
4. 88_L_111   (prob=0.712)
5. 88_L_155   (prob=0.842)

![L36 track A - Path calculation](log_29083/log_29083_L36-A-path.png)

We see a steady probability on all the path elements, even on the short connection between two switches on the main track:

![L36 track A - Path calculation details](log_29083/log_29083_L36-A-path-details.png)

#### Path projection

```bash
target/release/tp-cli.exe --gnss test-data/log_29083/log_29083_L36-A.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_29083/log_29083_L36-A-path-projection.geojson
```

No surprises. The blue dots are again the simple-projection result while the gold dots are the path processed coordinates:

![L36 track A - Path projection](static/log_29083_L36-A-path-projection.png)


---

Template

### Next

Log file ID: 

#### The GNSS data

![L36 track A - Raw GNSS](static/-raw.png)

#### Simple projection

```bash
target/release/tp-cli.exe simple-projection --gnss test-data/log_28876_L36-B.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_28876_L36-B-simple-projection.geojson
```
![L36 track B - Simple projection](static/-simple-projection.png)

#### Path calculation

```bash
target/release/tp-cli.exe --gnss test-data/log_28876_L36-B.csv calculate-path --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_28876_L36-B-path-calculation.geojson
```

![L36 track B - Path calculation](static/-path.png)

#### Path projection

```bash
target/release/tp-cli.exe --gnss test-data/log_28876_L36-B.csv --crs EPSG:4326 --network test-data/network_airport.geojson --output test-data/log_28876_L36-B-path-projection.geojson
```

![L36 track B - Path projection](static/-path-projection.png)