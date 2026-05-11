//! CLI entry point for tp-cli
//!
//! Provides three command modes:
//! - Default: Calculate train path and project coordinates
//! - calculate-path: Calculate train path only (no projection)
//! - simple-projection: Legacy nearest-netelement projection (feature 001)

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::process;
use tp_lib_core::{
    calculate_train_path, export_all_debug_info, parse_gnss_csv, parse_gnss_geojson,
    parse_network_geojson, parse_trainpath_csv, parse_trainpath_geojson, project_gnss,
    project_onto_path, write_csv, write_geojson, write_trainpath_csv, write_trainpath_geojson,
    Netelement, PathConfig, PathConfigBuilder, ProjectionConfig, RailwayNetwork,
};
#[cfg(feature = "webapp")]
use tp_webapp::{run_webapp_integrated, run_webapp_standalone, WebConfirmResult};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "tp-cli")]
#[command(about = "Train Positioning CLI - Calculate train paths and project GNSS positions onto railway networks", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // When no subcommand is provided, these are the default command arguments
    /// Path to GNSS input file (CSV or GeoJSON)
    #[arg(short = 'g', long = "gnss", value_name = "FILE", global = true)]
    gnss_file: Option<String>,

    /// CRS of GNSS data (required for CSV input, e.g., EPSG:4326)
    #[arg(long = "crs", value_name = "CRS", global = true)]
    gnss_crs: Option<String>,

    /// Path to railway network GeoJSON file
    #[arg(short = 'n', long = "network", value_name = "FILE", global = true)]
    network_file: Option<String>,

    /// Output file path (format determined by extension or --format)
    #[arg(short = 'o', long = "output", value_name = "FILE", global = true)]
    output_file: Option<String>,

    /// Pre-calculated train path file (skip path calculation)
    #[arg(long = "train-path", value_name = "FILE")]
    train_path_file: Option<String>,

    /// After path calculation open the webapp for visual review (integrated mode).
    /// Requires the `webapp` feature. The CLI continues only after the review is closed.
    #[cfg(feature = "webapp")]
    #[arg(long = "review")]
    review: bool,

    /// Save calculated path to this file (in addition to projected output)
    #[arg(long = "save-path", value_name = "FILE")]
    save_path_file: Option<String>,

    /// Output format (csv, geojson, or auto to detect from extension)
    #[arg(long = "format", value_name = "FORMAT", default_value = "auto")]
    format: String,

    // Algorithm parameters
    /// Distance exponential decay scale parameter (meters)
    #[arg(long = "distance-scale", value_name = "VALUE", default_value = "10.0")]
    distance_scale: f64,

    /// Heading exponential decay scale parameter (degrees)
    #[arg(long = "heading-scale", value_name = "VALUE", default_value = "2.0")]
    heading_scale: f64,

    /// Maximum distance for candidate selection (meters)
    #[arg(
        long = "cutoff-distance",
        value_name = "VALUE",
        default_value = "500.0"
    )]
    cutoff_distance: f64,

    /// Maximum heading difference before rejection (degrees)
    #[arg(long = "heading-cutoff", value_name = "VALUE", default_value = "10.0")]
    heading_cutoff: f64,

    /// Minimum probability for path segment inclusion
    #[arg(
        long = "probability-threshold",
        value_name = "VALUE",
        default_value = "0.02"
    )]
    probability_threshold: f64,

    /// Maximum candidate netelements per GNSS position
    #[arg(long = "max-candidates", value_name = "N", default_value = "3")]
    max_candidates: usize,

    /// Resample GNSS data at specified interval (meters) for path calculation
    #[arg(long = "resampling-distance", value_name = "VALUE")]
    resampling_distance: Option<f64>,

    /// Warning threshold for projection distance in meters (legacy parameter)
    #[arg(
        short = 'w',
        long = "warning-threshold",
        value_name = "METERS",
        default_value = "50.0"
    )]
    warning_threshold: f64,

    /// Latitude column name for CSV input
    #[arg(long = "lat-col", value_name = "COLUMN", default_value = "latitude")]
    lat_col: String,

    /// Longitude column name for CSV input
    #[arg(long = "lon-col", value_name = "COLUMN", default_value = "longitude")]
    lon_col: String,

    /// Timestamp column name for CSV input
    #[arg(long = "time-col", value_name = "COLUMN", default_value = "timestamp")]
    time_col: String,

    /// Enable debug mode: write intermediate GeoJSON files for each calculation phase
    #[arg(long = "debug", global = true)]
    debug: bool,

    /// Directory for debug output files (only used with --debug; defaults to output file directory)
    #[arg(long = "debug-output-dir", value_name = "DIR", global = true)]
    debug_output_dir: Option<String>,

    /// Enable verbose logging output
    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,

    /// Suppress all non-error output
    #[arg(long = "quiet", global = true)]
    quiet: bool,

    /// Path to a punctual detections file (CSV or GeoJSON). Detections are
    /// applied as anchors during path calculation (Feature 004).
    #[arg(
        long = "punctual-detections",
        value_name = "FILE",
        global = true
    )]
    punctual_detections: Option<String>,

    /// Path to a linear detections file (CSV or GeoJSON). Detections are applied as
    /// linear anchors during path calculation (Feature 004 US2).
    #[arg(
        long = "linear-detections",
        value_name = "FILE",
        global = true
    )]
    linear_detections: Option<String>,

    /// Maximum cutoff distance (meters) for resolving coordinate-only
    /// punctual detections to a netelement.
    #[arg(
        long = "cutoff-distance-detections",
        alias = "detection-cutoff",
        value_name = "METERS",
        default_value = "2.5",
        global = true
    )]
    detection_cutoff: f64,

    // Legacy argument for backward compatibility
    /// [DEPRECATED] Path to GNSS input file - use --gnss instead
    #[arg(long = "gnss-file", value_name = "FILE", hide = true)]
    legacy_gnss_file: Option<String>,

    /// [DEPRECATED] Path to network file - use --network instead
    #[arg(long = "network-file", value_name = "FILE", hide = true)]
    legacy_network_file: Option<String>,

    /// [DEPRECATED] Output format - use --format instead
    #[arg(long = "output-format", value_name = "FORMAT", hide = true)]
    legacy_output_format: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Calculate train path only without projecting coordinates
    #[command(name = "calculate-path")]
    CalculatePath {
        /// Path to GNSS input file (CSV or GeoJSON)
        #[arg(short = 'g', long = "gnss", value_name = "FILE")]
        gnss_file: String,

        /// CRS of GNSS data (required for CSV input)
        #[arg(long = "crs", value_name = "CRS")]
        gnss_crs: Option<String>,

        /// Path to railway network GeoJSON file
        #[arg(short = 'n', long = "network", value_name = "FILE")]
        network_file: String,

        /// Output file path for train path
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output_file: String,

        /// Output format (csv, geojson, or auto)
        #[arg(long = "format", value_name = "FORMAT", default_value = "auto")]
        format: String,

        // Algorithm parameters
        #[arg(long = "distance-scale", default_value = "10.0")]
        distance_scale: f64,
        #[arg(long = "heading-scale", default_value = "2.0")]
        heading_scale: f64,
        #[arg(long = "cutoff-distance", default_value = "500.0")]
        cutoff_distance: f64,
        #[arg(long = "heading-cutoff", default_value = "10.0")]
        heading_cutoff: f64,
        #[arg(long = "probability-threshold", default_value = "0.02")]
        probability_threshold: f64,
        #[arg(long = "max-candidates", default_value = "3")]
        max_candidates: usize,
        #[arg(long = "resampling-distance")]
        resampling_distance: Option<f64>,

        /// Enable debug mode: write intermediate GeoJSON files
        #[arg(long = "debug")]
        debug: bool,

        /// Directory for debug output files (only used with --debug; defaults to output file directory)
        #[arg(long = "debug-output-dir", value_name = "DIR")]
        debug_output_dir: Option<String>,

        /// Latitude column name for CSV
        #[arg(long = "lat-col", default_value = "latitude")]
        lat_col: String,
        /// Longitude column name for CSV
        #[arg(long = "lon-col", default_value = "longitude")]
        lon_col: String,
        /// Timestamp column name for CSV
        #[arg(long = "time-col", default_value = "timestamp")]
        time_col: String,
    },

    /// Legacy simple projection to nearest netelement (feature 001 behavior)
    #[command(name = "simple-projection")]
    SimpleProjection {
        /// Path to GNSS input file (CSV or GeoJSON)
        #[arg(short = 'g', long = "gnss", value_name = "FILE")]
        gnss_file: String,

        /// CRS of GNSS data (required for CSV input)
        #[arg(long = "crs", value_name = "CRS")]
        gnss_crs: Option<String>,

        /// Path to railway network GeoJSON file
        #[arg(short = 'n', long = "network", value_name = "FILE")]
        network_file: String,

        /// Output file path
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output_file: String,

        /// Output format (csv, geojson, or auto)
        #[arg(long = "format", value_name = "FORMAT", default_value = "auto")]
        format: String,

        /// Warning threshold for projection distance
        #[arg(short = 'w', long = "warning-threshold", default_value = "50.0")]
        warning_threshold: f64,

        /// Latitude column name for CSV
        #[arg(long = "lat-col", default_value = "latitude")]
        lat_col: String,
        /// Longitude column name for CSV
        #[arg(long = "lon-col", default_value = "longitude")]
        lon_col: String,
        /// Timestamp column name for CSV
        #[arg(long = "time-col", default_value = "timestamp")]
        time_col: String,
    },

    /// Launch the webapp to visually review and edit a pre-calculated train path.
    ///
    /// In standalone mode (default) the user saves the edited path to a CSV file.
    /// Requires the `webapp` feature (enabled by default).
    #[cfg(feature = "webapp")]
    #[command(name = "webapp")]
    Webapp {
        /// Path to railway network GeoJSON file
        #[arg(short = 'n', long = "network", value_name = "FILE")]
        network_file: String,

        /// Pre-calculated train path CSV file to review
        #[arg(long = "train-path", value_name = "FILE")]
        train_path_file: String,

        /// Output file for the reviewed path (standalone mode)
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output_file: Option<String>,

        /// Optional GNSS positions GeoJSON file to overlay on the map
        #[arg(long = "gnss", value_name = "FILE")]
        gnss_file: Option<String>,

        /// CRS of GNSS data (required for CSV input)
        #[arg(long = "crs", value_name = "CRS")]
        gnss_crs: Option<String>,

        /// Port to bind the server on (0 = scan default range 8765–8774)
        #[arg(long = "port", default_value = "0")]
        port: u16,

        /// Do not open the browser automatically after binding
        #[arg(long = "no-browser")]
        no_browser: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    // Initialize tracing subscriber based on verbose/quiet flags
    let filter = if cli.quiet {
        EnvFilter::new("error")
    } else if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("TP-CLI starting");

    // Route to appropriate command handler
    let result = match cli.command {
        Some(Commands::CalculatePath {
            gnss_file,
            gnss_crs,
            network_file,
            output_file,
            format,
            distance_scale,
            heading_scale,
            cutoff_distance,
            heading_cutoff,
            probability_threshold,
            max_candidates,
            resampling_distance,
            debug,
            debug_output_dir,
            lat_col,
            lon_col,
            time_col,
        }) => {
            if debug_output_dir.is_some() && !debug {
                tracing::warn!("--debug-output-dir is ignored without --debug");
                eprintln!("Warning: --debug-output-dir has no effect without --debug");
            }
            run_calculate_path(
                &gnss_file,
                gnss_crs.as_deref(),
                &network_file,
                &output_file,
                &format,
                distance_scale,
                heading_scale,
                cutoff_distance,
                heading_cutoff,
                probability_threshold,
                max_candidates,
                resampling_distance,
                debug,
                debug_output_dir.as_deref(),
                &lat_col,
                &lon_col,
                &time_col,
                cli.punctual_detections.as_deref(),
                cli.linear_detections.as_deref(),
                cli.detection_cutoff,
            )
        }
        Some(Commands::SimpleProjection {
            gnss_file,
            gnss_crs,
            network_file,
            output_file,
            format,
            warning_threshold,
            lat_col,
            lon_col,
            time_col,
        }) => run_simple_projection(
            &gnss_file,
            gnss_crs.as_deref(),
            &network_file,
            &output_file,
            &format,
            warning_threshold,
            &lat_col,
            &lon_col,
            &time_col,
        ),
        #[cfg(feature = "webapp")]
        Some(Commands::Webapp {
            network_file,
            train_path_file,
            output_file,
            gnss_file,
            gnss_crs,
            port,
            no_browser,
        }) => run_webapp_subcommand(
            &network_file,
            &train_path_file,
            output_file.as_deref(),
            gnss_file.as_deref(),
            gnss_crs.as_deref(),
            port,
            !no_browser,
        ),
        None => {
            // Default command: path-based projection
            // Handle legacy arguments for backward compatibility
            let gnss_file = cli.gnss_file.or(cli.legacy_gnss_file);
            let network_file = cli.network_file.or(cli.legacy_network_file);
            let format = cli.legacy_output_format.unwrap_or(cli.format);

            // Check if we have the required arguments
            let (gnss, network, output) = match (gnss_file, network_file, cli.output_file.clone()) {
                (Some(g), Some(n), Some(o)) => (g, n, o),
                (Some(g), Some(n), None) => {
                    // Legacy mode: output to stdout
                    run_legacy_pipeline(
                        &g,
                        cli.gnss_crs.as_deref(),
                        &n,
                        &format,
                        cli.warning_threshold,
                        &cli.lat_col,
                        &cli.lon_col,
                        &cli.time_col,
                    );
                    return;
                }
                _ => {
                    eprintln!(
                        "Error: Missing required arguments. Use --gnss, --network, and --output\n\
                         Run with --help for usage information."
                    );
                    process::exit(1);
                }
            };

            if cli.debug_output_dir.is_some() && !cli.debug {
                tracing::warn!("--debug-output-dir is ignored without --debug");
                eprintln!("Warning: --debug-output-dir has no effect without --debug");
            }
            run_default_command(
                &gnss,
                cli.gnss_crs.as_deref(),
                &network,
                &output,
                cli.train_path_file.as_deref(),
                cli.save_path_file.as_deref(),
                &format,
                cli.distance_scale,
                cli.heading_scale,
                cli.cutoff_distance,
                cli.heading_cutoff,
                cli.probability_threshold,
                cli.max_candidates,
                cli.resampling_distance,
                cli.debug,
                cli.debug_output_dir.as_deref(),
                cli.warning_threshold,
                &cli.lat_col,
                &cli.lon_col,
                &cli.time_col,
                {
                    #[cfg(feature = "webapp")]
                    {
                        cli.review
                    }
                    #[cfg(not(feature = "webapp"))]
                    {
                        false
                    }
                },
                cli.punctual_detections.as_deref(),
                cli.linear_detections.as_deref(),
                cli.detection_cutoff,
            )
        }
    };

    match result {
        Ok(()) => {
            tracing::info!("Pipeline completed successfully");
            process::exit(0)
        }
        Err(e) => {
            let exit_code = match e {
                PipelineError::Validation(_) => 1,
                PipelineError::Processing(_) => 2,
                PipelineError::Io(_) => 3,
            };
            tracing::error!(error = %e, exit_code = exit_code, "Pipeline failed");
            eprintln!("Error: {}", e);
            process::exit(exit_code);
        }
    }
}

#[derive(Debug)]
enum PipelineError {
    Validation(String),
    Processing(String),
    Io(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::Validation(msg) => write!(f, "Validation error: {}", msg),
            PipelineError::Processing(msg) => write!(f, "Processing error: {}", msg),
            PipelineError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

/// Load a pre-calculated train path, auto-detecting format from the file extension.
///
/// Files with a `.geojson` or `.json` extension are parsed with
/// [`parse_trainpath_geojson`]; all other files are assumed to be CSV and
/// parsed with [`parse_trainpath_csv`].
fn load_train_path(path: &str) -> Result<tp_lib_core::TrainPath, tp_lib_core::ProjectionError> {
    let lower = path.to_lowercase();
    if lower.ends_with(".geojson") || lower.ends_with(".json") {
        parse_trainpath_geojson(path)
    } else {
        parse_trainpath_csv(path)
    }
}

/// Determine output format from --format argument or file extension
fn determine_format(format: &str, output_path: &str) -> Result<&'static str, PipelineError> {
    match format.to_lowercase().as_str() {
        "csv" => Ok("csv"),
        "geojson" | "json" => Ok("json"),
        "auto" => {
            if output_path.ends_with(".csv") {
                Ok("csv")
            } else if output_path.ends_with(".geojson") || output_path.ends_with(".json") {
                Ok("json")
            } else {
                // Default to CSV if extension not recognized
                Ok("csv")
            }
        }
        _ => Err(PipelineError::Validation(format!(
            "Invalid format '{}'. Must be 'csv', 'geojson', or 'auto'",
            format
        ))),
    }
}

/// Derive the path output filename by inserting `-path` before the file extension.
///
/// Examples:
/// - `output.geojson` → `output-path.geojson`
/// - `output.csv`     → `output-path.csv`
/// - `output`         → `output-path`
fn derive_path_output(output_file: &str) -> String {
    match output_file.rfind('.') {
        Some(dot) => format!("{}-path{}", &output_file[..dot], &output_file[dot..]),
        None => format!("{}-path", output_file),
    }
}

/// Run the default command: calculate path and project coordinates
#[allow(clippy::too_many_arguments)]
fn run_default_command(
    gnss_file: &str,
    gnss_crs: Option<&str>,
    network_file: &str,
    output_file: &str,
    train_path_file: Option<&str>,
    save_path_file: Option<&str>,
    format: &str,
    distance_scale: f64,
    heading_scale: f64,
    cutoff_distance: f64,
    heading_cutoff: f64,
    probability_threshold: f64,
    max_candidates: usize,
    resampling_distance: Option<f64>,
    debug: bool,
    debug_output_dir: Option<&str>,
    _warning_threshold: f64,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
    review: bool,
    punctual_detections: Option<&str>,
    linear_detections: Option<&str>,
    detection_cutoff: f64,
) -> Result<(), PipelineError> {
    // Suppress unused warning when webapp feature is disabled
    #[cfg(not(feature = "webapp"))]
    let _ = review;

    let output_format = determine_format(format, output_file)?;

    // Load network
    tracing::info!(network_file = %network_file, "Loading railway network");
    let (netelements, netrelations) = parse_network_geojson(network_file)
        .map_err(|e| PipelineError::Io(format!("Failed to load network: {}", e)))?;
    tracing::info!(
        netelement_count = netelements.len(),
        netrelation_count = netrelations.len(),
        "Railway network loaded"
    );

    let network = RailwayNetwork::new(netelements.clone())
        .map_err(|e| PipelineError::Processing(format!("Failed to build network index: {}", e)))?;

    // Build netelement lookup map for write_trainpath_geojson
    let netelement_map: HashMap<String, Netelement> = netelements
        .iter()
        .map(|ne| (ne.id.clone(), ne.clone()))
        .collect();

    // Load GNSS positions
    let gnss_positions = load_gnss_positions(gnss_file, gnss_crs, lat_col, lon_col, time_col)?;
    tracing::info!(
        position_count = gnss_positions.len(),
        "GNSS positions loaded"
    );

    // Get or calculate train path
    let mut detection_provenance: Vec<tp_lib_core::DetectionRecord> = Vec::new();
    let mut train_path = if let Some(path_file) = train_path_file {
        // Use pre-calculated path
        tracing::info!(path_file = %path_file, "Loading pre-calculated train path");
        load_train_path(path_file)
            .map_err(|e| PipelineError::Io(format!("Failed to load train path: {}", e)))?
    } else {
        // Prepare detection anchors (Feature 004).
        let prepared_detections = prepare_detection_anchors(
            punctual_detections,
            linear_detections,
            &gnss_positions,
            &netelements,
            detection_cutoff,
        )?;

        // Calculate path
        let path_config = build_path_config(
            distance_scale,
            heading_scale,
            cutoff_distance,
            heading_cutoff,
            probability_threshold,
            max_candidates,
            resampling_distance,
            debug,
            prepared_detections
                .as_ref()
                .map(|p| p.anchors.clone())
                .unwrap_or_default(),
            detection_cutoff,
        )?;

        tracing::info!("Calculating train path");
        let mut result =
            calculate_train_path(&gnss_positions, &netelements, &netrelations, &path_config)
                .map_err(|e| {
                    PipelineError::Processing(format!("Path calculation failed: {}", e))
                })?;

        // Export debug info if debug mode was enabled
        if let Some(ref debug_info) = result.debug_info {
            let debug_dir = resolve_debug_dir(debug_output_dir, output_file);
            export_all_debug_info(debug_info, &debug_dir)
                .map_err(|e| PipelineError::Io(format!("Failed to write debug files: {}", e)))?;
            tracing::info!(debug_dir = %debug_dir, "Debug GeoJSON files written");
        }

        // Attach detection provenance and emit summary line (FR-017, FR-020).
        if let Some(prepared) = prepared_detections {
            emit_detection_summary(&prepared.records);
            for w in &prepared.warnings {
                tracing::warn!(warning = %w, "detection warning");
            }
            result.detection_provenance = prepared.records.clone();
            detection_provenance = prepared.records;
        }

        // Save path if requested
        if let Some(save_file) = save_path_file {
            if let Some(ref path) = result.path {
                let save_format = determine_format("auto", save_file)?;
                let mut file = File::create(save_file)
                    .map_err(|e| PipelineError::Io(format!("Failed to create path file: {}", e)))?;
                let mut writer = BufWriter::new(&mut file);
                match save_format {
                    "csv" => write_trainpath_csv(path, &mut writer).map_err(|e| {
                        PipelineError::Io(format!("Failed to write path CSV: {}", e))
                    })?,
                    _ => write_trainpath_geojson(path, &netelement_map, &mut writer).map_err(
                        |e| PipelineError::Io(format!("Failed to write path GeoJSON: {}", e)),
                    )?,
                }
                tracing::info!(save_file = %save_file, "Calculated path saved");
            }
        }

        result.path.ok_or_else(|| {
            PipelineError::Processing("Path calculation failed - no valid path found".to_string())
        })?
    };

    // Optional webapp review before projection (T029)
    #[cfg(feature = "webapp")]
    if review {
        tracing::info!("Launching path review webapp in integrated mode");
        match run_webapp_integrated(
            &network,
            train_path,
            Some(gnss_positions.clone()),
            detection_provenance.clone(),
            0,
            true,
        )
            .map_err(|e| PipelineError::Processing(format!("Webapp error: {}", e)))?
        {
            (WebConfirmResult::Confirmed, edited_path) => {
                tracing::info!(
                    "Review confirmed — continuing pipeline with (possibly edited) path"
                );
                train_path = edited_path;

                // Save the (possibly edited) path as <output_stem>-path.<ext>
                let path_output_file = derive_path_output(output_file);
                let path_save_format = determine_format("auto", &path_output_file)?;
                let mut path_file = File::create(&path_output_file).map_err(|e| {
                    PipelineError::Io(format!("Failed to create path output file: {}", e))
                })?;
                let mut path_writer = BufWriter::new(&mut path_file);
                match path_save_format {
                    "csv" => write_trainpath_csv(&train_path, &mut path_writer).map_err(|e| {
                        PipelineError::Io(format!("Failed to write path CSV: {}", e))
                    })?,
                    _ => write_trainpath_geojson(&train_path, &netelement_map, &mut path_writer)
                        .map_err(|e| {
                            PipelineError::Io(format!("Failed to write path GeoJSON: {}", e))
                        })?,
                }
                tracing::info!(path_output_file = %path_output_file, "Reviewed path saved");
                eprintln!("Path saved to: {}", path_output_file);
            }
            (WebConfirmResult::Aborted, _) => {
                tracing::info!("Review aborted — stopping pipeline");
                return Err(PipelineError::Processing(
                    "Pipeline aborted by user during review".to_string(),
                ));
            }
        }
    }

    // Build path config for projection (needed by project_onto_path)
    let path_config = build_path_config(
        distance_scale,
        heading_scale,
        cutoff_distance,
        heading_cutoff,
        probability_threshold,
        max_candidates,
        resampling_distance,
        false,
        Vec::new(),
        2.5,
    )?;

    // Project coordinates onto path
    tracing::info!("Projecting coordinates onto path");
    let projected = project_onto_path(&gnss_positions, &train_path, &netelements, &path_config)
        .map_err(|e| PipelineError::Processing(format!("Projection failed: {}", e)))?;
    tracing::info!(projected_count = projected.len(), "Projection completed");

    // Write output
    write_output(output_file, output_format, &projected)?;

    Ok(())
}

/// Run calculate-path subcommand: path only, no projection
#[allow(clippy::too_many_arguments)]
fn run_calculate_path(
    gnss_file: &str,
    gnss_crs: Option<&str>,
    network_file: &str,
    output_file: &str,
    format: &str,
    distance_scale: f64,
    heading_scale: f64,
    cutoff_distance: f64,
    heading_cutoff: f64,
    probability_threshold: f64,
    max_candidates: usize,
    resampling_distance: Option<f64>,
    debug: bool,
    debug_output_dir: Option<&str>,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
    punctual_detections: Option<&str>,
    linear_detections: Option<&str>,
    detection_cutoff: f64,
) -> Result<(), PipelineError> {
    let output_format = determine_format(format, output_file)?;

    // Load network
    tracing::info!(network_file = %network_file, "Loading railway network");
    let (netelements, netrelations) = parse_network_geojson(network_file)
        .map_err(|e| PipelineError::Io(format!("Failed to load network: {}", e)))?;
    tracing::info!(
        netelement_count = netelements.len(),
        netrelation_count = netrelations.len(),
        "Railway network loaded"
    );

    // Load GNSS positions
    let gnss_positions = load_gnss_positions(gnss_file, gnss_crs, lat_col, lon_col, time_col)?;
    tracing::info!(
        position_count = gnss_positions.len(),
        "GNSS positions loaded"
    );

    // Build path config
    let prepared_detections = prepare_detection_anchors(
        punctual_detections,
        linear_detections,
        &gnss_positions,
        &netelements,
        detection_cutoff,
    )?;
    let path_config = build_path_config(
        distance_scale,
        heading_scale,
        cutoff_distance,
        heading_cutoff,
        probability_threshold,
        max_candidates,
        resampling_distance,
        debug,
        prepared_detections
            .as_ref()
            .map(|p| p.anchors.clone())
            .unwrap_or_default(),
        detection_cutoff,
    )?;

    // Calculate path
    tracing::info!("Calculating train path");
    let result =
        calculate_train_path(&gnss_positions, &netelements, &netrelations, &path_config)
            .map_err(|e| PipelineError::Processing(format!("Path calculation failed: {}", e)))?;

    // Export debug info if debug mode was enabled
    if let Some(ref debug_info) = result.debug_info {
        let debug_dir = resolve_debug_dir(debug_output_dir, output_file);
        export_all_debug_info(debug_info, &debug_dir)
            .map_err(|e| PipelineError::Io(format!("Failed to write debug files: {}", e)))?;
        tracing::info!(debug_dir = %debug_dir, "Debug GeoJSON files written");
    }

    if let Some(prepared) = &prepared_detections {
        emit_detection_summary(&prepared.records);
        for w in &prepared.warnings {
            tracing::warn!(warning = %w, "detection warning");
        }
    }

    let path = result.path.ok_or_else(|| {
        PipelineError::Processing("Path calculation failed - no valid path found".to_string())
    })?;

    // Build netelement lookup map for write_trainpath_geojson
    let netelement_map: HashMap<String, Netelement> = netelements
        .iter()
        .map(|ne| (ne.id.clone(), ne.clone()))
        .collect();

    // Write path output
    let mut file = File::create(output_file)
        .map_err(|e| PipelineError::Io(format!("Failed to create output file: {}", e)))?;
    let mut writer = BufWriter::new(&mut file);

    match output_format {
        "csv" => write_trainpath_csv(&path, &mut writer)
            .map_err(|e| PipelineError::Io(format!("Failed to write CSV: {}", e)))?,
        _ => write_trainpath_geojson(&path, &netelement_map, &mut writer)
            .map_err(|e| PipelineError::Io(format!("Failed to write GeoJSON: {}", e)))?,
    }

    tracing::info!(output_file = %output_file, "Train path written");
    Ok(())
}

/// Run simple-projection subcommand: legacy nearest-netelement projection
#[allow(clippy::too_many_arguments)]
fn run_simple_projection(
    gnss_file: &str,
    gnss_crs: Option<&str>,
    network_file: &str,
    output_file: &str,
    format: &str,
    warning_threshold: f64,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
) -> Result<(), PipelineError> {
    let output_format = determine_format(format, output_file)?;

    // Load network (ignore netrelations for simple projection)
    tracing::info!(network_file = %network_file, "Loading railway network");
    let (netelements, _netrelations) = parse_network_geojson(network_file)
        .map_err(|e| PipelineError::Io(format!("Failed to load network: {}", e)))?;
    tracing::info!(
        netelement_count = netelements.len(),
        "Railway network loaded"
    );

    let network = RailwayNetwork::new(netelements)
        .map_err(|e| PipelineError::Processing(format!("Failed to build network index: {}", e)))?;

    // Load GNSS positions
    let gnss_positions = load_gnss_positions(gnss_file, gnss_crs, lat_col, lon_col, time_col)?;
    tracing::info!(
        position_count = gnss_positions.len(),
        "GNSS positions loaded"
    );

    // Project using simple nearest-netelement method
    let config = ProjectionConfig {
        projection_distance_warning_threshold: warning_threshold,
        suppress_warnings: false,
    };

    tracing::info!("Starting simple projection");
    let projected = project_gnss(&gnss_positions, &network, &config)
        .map_err(|e| PipelineError::Processing(format!("Projection failed: {}", e)))?;
    tracing::info!(projected_count = projected.len(), "Projection completed");

    // Write output
    write_output(output_file, output_format, &projected)?;

    Ok(())
}

/// Legacy pipeline for backward compatibility (output to stdout)
#[allow(clippy::too_many_arguments)]
fn run_legacy_pipeline(
    gnss_file: &str,
    gnss_crs: Option<&str>,
    network_file: &str,
    format: &str,
    warning_threshold: f64,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
) {
    // This matches the original behavior exactly
    let (netelements, _) = match parse_network_geojson(network_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: Failed to load network: {}", e);
            process::exit(3);
        }
    };

    let network = match RailwayNetwork::new(netelements) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error: Failed to build network index: {}", e);
            process::exit(2);
        }
    };

    let gnss_positions = match load_gnss_positions(gnss_file, gnss_crs, lat_col, lon_col, time_col)
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(3);
        }
    };

    let config = ProjectionConfig {
        projection_distance_warning_threshold: warning_threshold,
        suppress_warnings: false,
    };

    let projected = match project_gnss(&gnss_positions, &network, &config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: Projection failed: {}", e);
            process::exit(2);
        }
    };

    // Write to stdout
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();

    let output_format = match format.to_lowercase().as_str() {
        "csv" => "csv",
        _ => "json",
    };

    let result = match output_format {
        "csv" => write_csv(&projected, &mut writer),
        _ => write_geojson(&projected, &mut writer),
    };

    if let Err(e) = result {
        eprintln!("Error: Failed to write output: {}", e);
        process::exit(3);
    }
}

/// Load GNSS positions from file
fn load_gnss_positions(
    gnss_file: &str,
    gnss_crs: Option<&str>,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
) -> Result<Vec<tp_lib_core::GnssPosition>, PipelineError> {
    tracing::info!(gnss_file = %gnss_file, "Loading GNSS positions");

    if gnss_file.ends_with(".csv") {
        let crs = gnss_crs.ok_or_else(|| {
            PipelineError::Validation("--crs is required for CSV input".to_string())
        })?;
        parse_gnss_csv(gnss_file, crs, lat_col, lon_col, time_col)
            .map_err(|e| PipelineError::Io(format!("Failed to load GNSS data: {}", e)))
    } else if gnss_file.ends_with(".geojson") || gnss_file.ends_with(".json") {
        let crs = gnss_crs.unwrap_or("EPSG:4326");
        parse_gnss_geojson(gnss_file, crs)
            .map_err(|e| PipelineError::Io(format!("Failed to load GNSS GeoJSON: {}", e)))
    } else {
        Err(PipelineError::Validation(format!(
            "Unsupported GNSS file format: {}. Use .csv, .geojson, or .json",
            gnss_file
        )))
    }
}

/// Prepare detection anchors from optional punctual and/or linear detection
/// files (Feature 004). Returns the merged [`PreparedDetections`] when at
/// least one file is supplied; returns `None` otherwise.
fn prepare_detection_anchors(
    punctual_detections: Option<&str>,
    linear_detections: Option<&str>,
    gnss: &[tp_lib_core::GnssPosition],
    netelements: &[Netelement],
    detection_cutoff: f64,
) -> Result<Option<tp_lib_core::PreparedDetections>, PipelineError> {
    if punctual_detections.is_none() && linear_detections.is_none() {
        return Ok(None);
    }

    let mut merged = tp_lib_core::PreparedDetections {
        anchors: Vec::new(),
        records: Vec::new(),
        warnings: Vec::new(),
    };

    if let Some(path) = punctual_detections {
        let prepared = tp_lib_core::prepare_detections(
            std::path::Path::new(path),
            tp_lib_core::DetectionKind::Punctual,
            gnss,
            netelements,
            detection_cutoff,
        )
        .map_err(|e| PipelineError::Validation(format!("detections: {}", e)))?;
        merged.anchors.extend(prepared.anchors);
        merged.records.extend(prepared.records);
        merged.warnings.extend(prepared.warnings);
    }

    if let Some(path) = linear_detections {
        let prepared = tp_lib_core::prepare_detections(
            std::path::Path::new(path),
            tp_lib_core::DetectionKind::Linear,
            gnss,
            netelements,
            detection_cutoff,
        )
        .map_err(|e| PipelineError::Validation(format!("detections: {}", e)))?;
        merged.anchors.extend(prepared.anchors);
        merged.records.extend(prepared.records);
        merged.warnings.extend(prepared.warnings);
    }

    // Sort merged anchors by first GNSS index (ascending) per FR-013/FR-019.
    merged
        .anchors
        .sort_by_key(|a| a.first_index());

    Ok(Some(merged))
}

/// Emit the FR-020 stderr summary line:
/// `"detections: N applied, M discarded (breakdown)"`.
fn emit_detection_summary(records: &[tp_lib_core::DetectionRecord]) {
    use tp_lib_core::{DetectionStatus, DiscardReason};
    let mut applied = 0usize;
    let mut discarded = 0usize;
    let mut out_of_time = 0usize;
    let mut out_of_reach = 0usize;
    let mut unknown_ne = 0usize;
    let mut intrinsic_oor = 0usize;
    let mut duplicate = 0usize;
    for r in records {
        match &r.status {
            DetectionStatus::Applied { .. } | DetectionStatus::Resolved { .. } => applied += 1,
            DetectionStatus::Discarded { reason } => {
                discarded += 1;
                match reason {
                    DiscardReason::OutOfTimeRange { .. } => out_of_time += 1,
                    DiscardReason::OutOfReach { .. } => out_of_reach += 1,
                    DiscardReason::UnknownNetelement { .. } => unknown_ne += 1,
                    DiscardReason::IntrinsicOutOfRange { .. } => intrinsic_oor += 1,
                    DiscardReason::DuplicateOfPriorDetection { .. } => duplicate += 1,
                }
            }
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if out_of_time > 0 {
        parts.push(format!("{} out_of_time_range", out_of_time));
    }
    if out_of_reach > 0 {
        parts.push(format!("{} out_of_reach", out_of_reach));
    }
    if unknown_ne > 0 {
        parts.push(format!("{} unknown_netelement", unknown_ne));
    }
    if intrinsic_oor > 0 {
        parts.push(format!("{} intrinsic_out_of_range", intrinsic_oor));
    }
    if duplicate > 0 {
        parts.push(format!("{} duplicate", duplicate));
    }
    let breakdown = if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    };
    eprintln!(
        "detections: {} applied, {} discarded{}",
        applied, discarded, breakdown
    );
}

/// Build PathConfig from CLI parameters
#[allow(clippy::too_many_arguments)]
fn build_path_config(
    distance_scale: f64,
    heading_scale: f64,
    cutoff_distance: f64,
    heading_cutoff: f64,
    probability_threshold: f64,
    max_candidates: usize,
    resampling_distance: Option<f64>,
    debug_mode: bool,
    anchors: Vec<tp_lib_core::ResolvedAnchor>,
    detection_cutoff_distance: f64,
) -> Result<PathConfig, PipelineError> {
    let builder = PathConfigBuilder::default()
        .distance_scale(distance_scale)
        .heading_scale(heading_scale)
        .cutoff_distance(cutoff_distance)
        .heading_cutoff(heading_cutoff)
        .probability_threshold(probability_threshold)
        .max_candidates(max_candidates)
        .resampling_distance(resampling_distance)
        .debug_mode(debug_mode)
        .anchors(anchors)
        .detection_cutoff_distance(detection_cutoff_distance);

    builder
        .build()
        .map_err(|e| PipelineError::Validation(format!("Invalid path configuration: {}", e)))
}

/// Resolve debug output directory from CLI argument, defaulting to a "debug" subdirectory
/// inside the output file's parent directory
fn resolve_debug_dir(debug_output_dir: Option<&str>, output_file: &str) -> String {
    match debug_output_dir {
        Some(dir) if !dir.is_empty() => dir.to_string(),
        _ => {
            let path = std::path::Path::new(output_file);
            let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            parent.join("debug").to_string_lossy().into_owned()
        }
    }
}

/// Write projected positions to output file
fn write_output(
    output_file: &str,
    format: &str,
    projected: &[tp_lib_core::ProjectedPosition],
) -> Result<(), PipelineError> {
    let mut file = File::create(output_file)
        .map_err(|e| PipelineError::Io(format!("Failed to create output file: {}", e)))?;
    let mut writer = BufWriter::new(&mut file);

    match format {
        "csv" => write_csv(projected, &mut writer)
            .map_err(|e| PipelineError::Io(format!("Failed to write CSV: {}", e)))?,
        _ => write_geojson(projected, &mut writer)
            .map_err(|e| PipelineError::Io(format!("Failed to write GeoJSON: {}", e)))?,
    }

    tracing::info!(output_file = %output_file, "Output written");
    Ok(())
}

/// Launch the webapp in standalone mode for visual path review (T022, T034)
#[cfg(feature = "webapp")]
#[allow(clippy::too_many_arguments)]
fn run_webapp_subcommand(
    network_file: &str,
    train_path_file: &str,
    output_file: Option<&str>,
    gnss_file: Option<&str>,
    gnss_crs: Option<&str>,
    port: u16,
    open_browser: bool,
) -> Result<(), PipelineError> {
    // Load network
    tracing::info!(network_file = %network_file, "Loading railway network for webapp");
    let (netelements, _netrelations) = parse_network_geojson(network_file)
        .map_err(|e| PipelineError::Io(format!("Failed to load network: {}", e)))?;
    let network = RailwayNetwork::new(netelements)
        .map_err(|e| PipelineError::Processing(format!("Failed to build network index: {}", e)))?;

    // Load train path
    tracing::info!(train_path_file = %train_path_file, "Loading train path for webapp");
    let path = load_train_path(train_path_file)
        .map_err(|e| PipelineError::Io(format!("Failed to load train path: {}", e)))?;

    // Load optional GNSS positions (T034)
    let gnss = if let Some(gf) = gnss_file {
        let positions = load_gnss_positions(gf, gnss_crs, "latitude", "longitude", "timestamp")?;
        tracing::info!(
            position_count = positions.len(),
            "GNSS positions loaded for webapp"
        );
        Some(positions)
    } else {
        None
    };

    let output_path = output_file.map(std::path::PathBuf::from);

    tracing::info!(
        port = port,
        "Launching path review webapp in standalone mode"
    );
    run_webapp_standalone(&network, path, output_path, gnss, Vec::new(), port, open_browser)
        .map_err(|e| PipelineError::Processing(format!("Webapp error: {}", e)))?;

    Ok(())
}
