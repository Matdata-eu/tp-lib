//! CLI entry point for tp-cli

use clap::Parser;
use std::process;
use tp_lib_core::{
    parse_gnss_csv, parse_gnss_geojson, parse_network_geojson, project_gnss, write_csv,
    write_geojson, ProjectionConfig, RailwayNetwork,
};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "tp-cli")]
#[command(about = "Train Positioning CLI - Project GNSS positions onto railway track netelements", long_about = None)]
#[command(version)]
struct Args {
    /// Path to GNSS input file (CSV or GeoJSON)
    #[arg(short = 'g', long = "gnss-file", value_name = "FILE")]
    gnss_file: String,

    /// CRS of GNSS data (required for CSV input, e.g., EPSG:4326)
    #[arg(long = "crs", value_name = "CRS")]
    gnss_crs: Option<String>,

    /// Path to railway network GeoJSON file
    #[arg(short = 'n', long = "network-file", value_name = "FILE")]
    network_file: String,

    /// Output format (csv or json)
    #[arg(
        short = 'o',
        long = "output-format",
        value_name = "FORMAT",
        default_value = "json"
    )]
    output_format: String,

    /// Warning threshold for projection distance in meters
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
}

fn main() {
    // Initialize tracing subscriber for structured logging
    // Respects RUST_LOG environment variable for log level control
    // Default: error level, can be overridden with RUST_LOG=debug, RUST_LOG=info, etc.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    tracing::info!("TP-CLI starting");
    tracing::debug!(?args, "Parsed command-line arguments");

    // Validate arguments
    if let Err(e) = validate_args(&args) {
        tracing::error!(error = %e, "Argument validation failed");
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    // Run the projection pipeline
    match run_pipeline(args) {
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

fn validate_args(args: &Args) -> Result<(), String> {
    // Check if GNSS file is CSV and requires CRS
    if args.gnss_file.ends_with(".csv") {
        if args.gnss_crs.is_none() {
            return Err("--crs is required for CSV input".to_string());
        }
    } else if args.gnss_file.ends_with(".geojson") || args.gnss_file.ends_with(".json") {
        // GeoJSON files include their CRS in the file
        if args.gnss_crs.is_some() {
            eprintln!(
                "Warning: --crs argument is ignored for GeoJSON files (CRS is always EPSG:4326)"
            );
        }
    } else {
        return Err(format!(
            "Unsupported GNSS file format. Must be .csv, .geojson, or .json: {}",
            args.gnss_file
        ));
    }

    // Validate output format
    if args.output_format != "csv" && args.output_format != "json" {
        return Err(format!(
            "Invalid output format '{}'. Must be 'csv' or 'json'",
            args.output_format
        ));
    }

    // Validate warning threshold
    if args.warning_threshold < 0.0 {
        return Err("Warning threshold must be non-negative".to_string());
    }

    Ok(())
}

fn run_pipeline(args: Args) -> Result<(), PipelineError> {
    // Load railway network
    tracing::info!(network_file = %args.network_file, "Loading railway network");
    let (netelements, netrelations) = parse_network_geojson(&args.network_file)
        .map_err(|e| PipelineError::Io(format!("Failed to load network: {}", e)))?;
    tracing::info!(
        netelement_count = netelements.len(),
        netrelation_count = netrelations.len(),
        "Railway network loaded"
    );

    let network = RailwayNetwork::new(netelements)
        .map_err(|e| PipelineError::Processing(format!("Failed to build network index: {}", e)))?;
    tracing::debug!("Spatial index built successfully");

    // Load GNSS positions
    tracing::info!(gnss_file = %args.gnss_file, "Loading GNSS positions");
    let gnss_positions = if args.gnss_file.ends_with(".csv") {
        let crs = args.gnss_crs.as_ref().unwrap(); // Already validated
        tracing::debug!(crs = %crs, "Parsing CSV with CRS");
        parse_gnss_csv(
            &args.gnss_file,
            crs,
            &args.lat_col,
            &args.lon_col,
            &args.time_col,
        )
        .map_err(|e| PipelineError::Io(format!("Failed to load GNSS data: {}", e)))?
    } else if args.gnss_file.ends_with(".geojson") || args.gnss_file.ends_with(".json") {
        // For GeoJSON, use the CRS from the file if specified, otherwise default to EPSG:4326
        let crs = args.gnss_crs.as_deref().unwrap_or("EPSG:4326");
        tracing::debug!(crs = %crs, "Parsing GeoJSON with CRS");
        parse_gnss_geojson(&args.gnss_file, crs)
            .map_err(|e| PipelineError::Io(format!("Failed to load GNSS GeoJSON data: {}", e)))?
    } else {
        return Err(PipelineError::Validation(
            "Unsupported GNSS file format. Use .csv, .geojson, or .json".to_string(),
        ));
    };
    tracing::info!(
        position_count = gnss_positions.len(),
        "GNSS positions loaded"
    );

    // Project GNSS positions onto network
    let config = ProjectionConfig {
        projection_distance_warning_threshold: args.warning_threshold,
        suppress_warnings: false,
    };
    tracing::debug!(
        warning_threshold = config.projection_distance_warning_threshold,
        "Projection configuration"
    );

    tracing::info!("Starting projection");
    let projected = project_gnss(&gnss_positions, &network, &config)
        .map_err(|e| PipelineError::Processing(format!("Projection failed: {}", e)))?;
    tracing::info!(projected_count = projected.len(), "Projection completed");

    // Write output to stdout
    tracing::info!(output_format = %args.output_format, "Writing output");
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();

    match args.output_format.as_str() {
        "csv" => {
            write_csv(&projected, &mut writer)
                .map_err(|e| PipelineError::Io(format!("Failed to write CSV output: {}", e)))?;
            tracing::debug!("CSV output written successfully");
        }
        "json" => {
            write_geojson(&projected, &mut writer)
                .map_err(|e| PipelineError::Io(format!("Failed to write GeoJSON output: {}", e)))?;
            tracing::debug!("GeoJSON output written successfully");
        }
        _ => unreachable!("Output format already validated"),
    }

    Ok(())
}
