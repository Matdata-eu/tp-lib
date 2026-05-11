//! Detection time-range filter (T013).
//!
//! Discards detections whose timestamps fall outside the GNSS observation
//! window. Linear detections that *partially* overlap (FR-008) are also
//! discarded — no clipping.

use chrono::{DateTime, FixedOffset, TimeZone, Utc};

use crate::models::{
    Detection, DetectionKind, DetectionRecord, DetectionStatus, DiscardReason, GnssPosition,
    TimestampOrRange,
};

/// Output of [`filter_detections_by_time`]: kept detections plus discard records.
#[derive(Debug, Clone, Default)]
pub struct FilterOutcome {
    pub kept: Vec<Detection>,
    pub discard_records: Vec<DetectionRecord>,
    pub warnings: Vec<String>,
}

/// Filter detections by GNSS observation window.
pub fn filter_detections_by_time(
    detections: Vec<Detection>,
    gnss: &[GnssPosition],
) -> FilterOutcome {
    if gnss.is_empty() {
        // No window → discard everything as out-of-time-range using sentinels.
        let dummy = Utc
            .timestamp_opt(0, 0)
            .single()
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        return discard_all(detections, dummy, dummy);
    }

    let gnss_first = gnss.first().unwrap().timestamp;
    let gnss_last = gnss.last().unwrap().timestamp;
    // Defensive: handle un-sorted by computing min/max.
    let (gnss_first, gnss_last) = gnss
        .iter()
        .map(|g| g.timestamp)
        .fold((gnss_first, gnss_last), |(lo, hi), t| {
            (lo.min(t), hi.max(t))
        });

    let mut out = FilterOutcome::default();
    for det in detections.into_iter() {
        let in_window = match &det {
            Detection::Punctual(p) => p.timestamp >= gnss_first && p.timestamp <= gnss_last,
            Detection::Linear(l) => l.t_from >= gnss_first && l.t_to <= gnss_last,
        };
        if in_window {
            out.kept.push(det);
        } else {
            out.discard_records
                .push(make_discard_record(&det, gnss_first, gnss_last));
            out.warnings.push(format!(
                "detection at {}:{} discarded (outside GNSS window [{}, {}])",
                det.source_file(),
                det.source_row(),
                gnss_first,
                gnss_last,
            ));
        }
    }
    out
}

fn make_discard_record(
    det: &Detection,
    gnss_first: DateTime<FixedOffset>,
    gnss_last: DateTime<FixedOffset>,
) -> DetectionRecord {
    let reason = DiscardReason::OutOfTimeRange {
        gnss_first,
        gnss_last,
    };
    match det {
        Detection::Punctual(p) => DetectionRecord {
            source_file: p.source_file.clone(),
            source_row: p.source_row,
            kind: DetectionKind::Punctual,
            timestamp: TimestampOrRange::Single {
                timestamp: p.timestamp,
            },
            status: DetectionStatus::Discarded { reason },
            id: p.id.clone(),
            source: p.source.clone(),
            metadata: p.metadata.clone(),
        },
        Detection::Linear(l) => DetectionRecord {
            source_file: l.source_file.clone(),
            source_row: l.source_row,
            kind: DetectionKind::Linear,
            timestamp: TimestampOrRange::Range {
                t_from: l.t_from,
                t_to: l.t_to,
            },
            status: DetectionStatus::Discarded { reason },
            id: l.id.clone(),
            source: l.source.clone(),
            metadata: l.metadata.clone(),
        },
    }
}

fn discard_all(
    detections: Vec<Detection>,
    gnss_first: DateTime<FixedOffset>,
    gnss_last: DateTime<FixedOffset>,
) -> FilterOutcome {
    let mut out = FilterOutcome::default();
    for det in detections.into_iter() {
        out.warnings.push(format!(
            "detection at {}:{} discarded (empty GNSS track)",
            det.source_file(),
            det.source_row(),
        ));
        out.discard_records
            .push(make_discard_record(&det, gnss_first, gnss_last));
    }
    out
}
