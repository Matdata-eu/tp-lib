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
    calculate_train_path, parse_gnss_csv, parse_gnss_geojson, parse_network_geojson,
    parse_trainpath_csv, project_gnss, project_onto_path, write_csv, write_geojson,
    write_trainpath_csv, write_trainpath_geojson, Netelement, PathConfig, PathConfigBuilder,
    ProjectionConfig, RailwayNetwork,
};
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
    #[arg(long = "cutoff-distance", value_name = "VALUE", default_value = "50.0")]
    cutoff_distance: f64,

    /// Maximum heading difference before rejection (degrees)
    #[arg(long = "heading-cutoff", value_name = "VALUE", default_value = "5.0")]
    heading_cutoff: f64,

    /// Minimum probability for path segment inclusion
    #[arg(
        long = "probability-threshold",
        value_name = "VALUE",
        default_value = "0.25"
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

    /// Enable verbose logging output
    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,

    /// Suppress all non-error output
    #[arg(long = "quiet", global = true)]
    quiet: bool,

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
        #[arg(long = "cutoff-distance", default_value = "50.0")]
        cutoff_distance: f64,
        #[arg(long = "heading-cutoff", default_value = "5.0")]
        heading_cutoff: f64,
        #[arg(long = "probability-threshold", default_value = "0.25")]
        probability_threshold: f64,
        #[arg(long = "max-candidates", default_value = "3")]
        max_candidates: usize,
        #[arg(long = "resampling-distance")]
        resampling_distance: Option<f64>,

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
            lat_col,
            lon_col,
            time_col,
        }) => run_calculate_path(
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
            &lat_col,
            &lon_col,
            &time_col,
        ),
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
                cli.warning_threshold,
                &cli.lat_col,
                &cli.lon_col,
                &cli.time_col,
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
    _warning_threshold: f64,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
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

    let _network = RailwayNetwork::new(netelements.clone())
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
    let train_path = if let Some(path_file) = train_path_file {
        // Use pre-calculated path
        tracing::info!(path_file = %path_file, "Loading pre-calculated train path");
        parse_trainpath_csv(path_file)
            .map_err(|e| PipelineError::Io(format!("Failed to load train path: {}", e)))?
    } else {
        // Calculate path
        let path_config = build_path_config(
            distance_scale,
            heading_scale,
            cutoff_distance,
            heading_cutoff,
            probability_threshold,
            max_candidates,
            resampling_distance,
        )?;

        tracing::info!("Calculating train path");
        let result =
            calculate_train_path(&gnss_positions, &netelements, &netrelations, &path_config)
                .map_err(|e| {
                    PipelineError::Processing(format!("Path calculation failed: {}", e))
                })?;

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

    // Build path config for projection (needed by project_onto_path)
    let path_config = build_path_config(
        distance_scale,
        heading_scale,
        cutoff_distance,
        heading_cutoff,
        probability_threshold,
        max_candidates,
        resampling_distance,
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
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
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
    let path_config = build_path_config(
        distance_scale,
        heading_scale,
        cutoff_distance,
        heading_cutoff,
        probability_threshold,
        max_candidates,
        resampling_distance,
    )?;

    // Calculate path
    tracing::info!("Calculating train path");
    let result =
        calculate_train_path(&gnss_positions, &netelements, &netrelations, &path_config)
            .map_err(|e| PipelineError::Processing(format!("Path calculation failed: {}", e)))?;

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

/// Build PathConfig from CLI parameters
fn build_path_config(
    distance_scale: f64,
    heading_scale: f64,
    cutoff_distance: f64,
    heading_cutoff: f64,
    probability_threshold: f64,
    max_candidates: usize,
    resampling_distance: Option<f64>,
) -> Result<PathConfig, PipelineError> {
    let builder = PathConfigBuilder::default()
        .distance_scale(distance_scale)
        .heading_scale(heading_scale)
        .cutoff_distance(cutoff_distance)
        .heading_cutoff(heading_cutoff)
        .probability_threshold(probability_threshold)
        .max_candidates(max_candidates)
        .resampling_distance(resampling_distance);

    builder
        .build()
        .map_err(|e| PipelineError::Validation(format!("Invalid path configuration: {}", e)))
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
