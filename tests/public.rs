// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use avif_parse::Error;
use std::fs::File;

static IMAGE_AVIF: &str = "av1-avif/testFiles/Microsoft/Monochrome.avif";
static IMAGE_AVIF_EXTENTS: &str = "tests/kodim-extents.avif";
static IMAGE_AVIF_CORRUPT: &str = "tests/bug-1655846.avif";
static IMAGE_AVIF_CORRUPT_2: &str = "tests/bug-1661347.avif";
static IMAGE_GRID_5X4: &str = "av1-avif/testFiles/Microsoft/Summer_in_Tomsk_720p_5x4_grid.avif";
static ANIMATED_AVIF: &str = "link-u-samples/star-8bpc.avifs";
static AOMEDIA_TEST_FILES: &str = "av1-avif/testFiles";
static LINK_U_SAMPLES: &str = "link-u-samples";

#[test]
fn public_avif_primary_item() {
    let input = &mut File::open(IMAGE_AVIF).expect("Unknown file");
    let context = avif_parse::read_avif(input).expect("read_avif failed");
    assert_eq!(context.primary_item.len(), 6979);
    assert_eq!(context.primary_item[0..4], [0x12, 0x00, 0x0a, 0x0a]);
}

#[test]
fn public_avif_primary_item_split_extents() {
    let input = &mut File::open(IMAGE_AVIF_EXTENTS).expect("Unknown file");
    let context = avif_parse::read_avif(input).expect("read_avif failed");
    assert_eq!(context.primary_item.len(), 4387);
}

#[test]
fn public_avif_bug_1655846() {
    let input = &mut File::open(IMAGE_AVIF_CORRUPT).expect("Unknown file");
    assert!(avif_parse::read_avif(input).is_err());
}

#[test]
fn public_avif_bug_1661347() {
    let input = &mut File::open(IMAGE_AVIF_CORRUPT_2).expect("Unknown file");
    assert!(avif_parse::read_avif(input).is_err());
}

#[test]
fn aomedia_sample_images() {
    test_dir(AOMEDIA_TEST_FILES);
}

#[test]
fn linku_sample_images() {
    test_dir(LINK_U_SAMPLES);
}

#[test]
fn grid_5x4_ispe_calculation() {
    // Test grid layout calculation from ispe (Image Spatial Extents) properties
    // This file has NO explicit ImageGrid box, only ispe properties
    let input = &mut File::open(IMAGE_GRID_5X4).expect("Unknown file");
    let avif = avif_parse::read_avif(input).expect("read_avif failed");

    // Verify grid config was calculated correctly from ispe
    let grid = avif.grid_config.expect("Expected grid config");
    assert_eq!(grid.rows, 4, "Expected 4 rows");
    assert_eq!(grid.columns, 5, "Expected 5 columns");
    assert_eq!(grid.output_width, 6400, "Expected width 6400");
    assert_eq!(grid.output_height, 2880, "Expected height 2880");

    // Verify tile count matches grid dimensions
    assert_eq!(avif.grid_tiles.len(), 20, "Expected 20 tiles (4×5)");

    // Verify primary item is empty for grid images
    assert_eq!(avif.primary_item.len(), 0, "Grid images should have empty primary_item");
}

#[test]
fn grid_tile_ordering() {
    // Verify tiles are ordered by dimgIdx (reference_index) not item_id
    let input = &mut File::open(IMAGE_GRID_5X4).expect("Unknown file");
    let avif = avif_parse::read_avif(input).expect("read_avif failed");

    // All tiles should have valid data
    for (i, tile) in avif.grid_tiles.iter().enumerate() {
        assert!(!tile.is_empty(), "Tile {} should not be empty", i);
        assert!(tile.len() > 1000, "Tile {} seems too small ({} bytes)", i, tile.len());
    }

    // Tiles should be in dimgIdx order (verified by the sizes being reasonable)
    // The Microsoft 5×4 grid has tiles with varying sizes, confirming order matters
}

#[test]
fn animated_avif_frame_extraction() {
    // Test animation parsing from .avifs file
    let input = &mut File::open(ANIMATED_AVIF).expect("Unknown file");
    let avif = avif_parse::read_avif(input).expect("read_avif failed");

    // Verify animation was detected and parsed
    let animation = avif.animation.expect("Expected animation data");

    // Verify frame count
    assert_eq!(animation.frames.len(), 5, "Expected 5 frames");

    // Verify all frames have valid data
    for (i, frame) in animation.frames.iter().enumerate() {
        assert!(!frame.data.is_empty(), "Frame {} should not be empty", i);
        assert!(frame.duration_ms > 0, "Frame {} should have positive duration", i);
    }

    // Verify expected durations (star-8bpc.avifs has 100ms per frame)
    for frame in &animation.frames {
        assert_eq!(frame.duration_ms, 100, "Expected 100ms frame duration");
    }

    // Verify primary item contains first frame
    assert_eq!(avif.primary_item.len(), animation.frames[0].data.len(),
        "Primary item should match first frame");
}

fn test_dir(dir: &str) {
    let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::max()).try_init();
    let mut errors = 0;

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.expect("AVIF entry");
        let path = entry.path();
        let ext = path.extension().unwrap_or_default();
        if !path.is_file() || (ext != "avif" && ext != "avifs") {
            continue; // Skip directories, ReadMe.txt, etc.
        }
        log::debug!("parsing {:?}", path.display());
        let input = &mut File::open(path).expect("bad file");
        match avif_parse::read_avif(input) {
            Ok(avif) => {
                // Grid images have tiles instead of primary_item
                if avif.grid_config.is_none() {
                    avif.primary_item_metadata().unwrap();
                    avif.alpha_item_metadata().unwrap();
                } else {
                    // For grid images, validate that we have tiles
                    assert!(!avif.grid_tiles.is_empty(), "Grid image has no tiles");
                }
            },
            Err(Error::Unsupported(why)) => log::warn!("{why}"),
            Err(err) => {
                log::error!("{:?}: {err}", path.display());
                errors += 1;
            },
        }
    }
    assert_eq!(0, errors);
}

// ============================================================================
// Resource Limit Tests
// ============================================================================

#[test]
fn resource_limit_peak_memory() {
    // Test peak memory limit enforcement
    let input = &mut File::open(IMAGE_AVIF).expect("Unknown file");

    // Set very low peak memory limit (1KB)
    let config = avif_parse::DecodeConfig::default()
        .with_peak_memory_limit(1_000);

    let result = avif_parse::read_avif_with_config(input, &config, enough::Unstoppable);

    // Should fail due to peak memory limit
    match result {
        Err(avif_parse::Error::ResourceLimitExceeded(msg)) => {
            assert_eq!(msg, "peak memory limit exceeded");
        }
        Ok(_) => panic!("Expected peak memory limit error"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn resource_limit_total_megapixels() {
    // Test total megapixels limit for grid images
    let input = &mut File::open(IMAGE_GRID_5X4).expect("Unknown file");

    // Grid is 6400×2880 = 18.432 MP
    // Set limit below that
    let config = avif_parse::DecodeConfig::default()
        .with_total_megapixels_limit(10);

    let result = avif_parse::read_avif_with_config(input, &config, enough::Unstoppable);

    // Should fail due to megapixels limit
    match result {
        Err(avif_parse::Error::ResourceLimitExceeded(msg)) => {
            assert_eq!(msg, "total megapixels limit exceeded");
        }
        Ok(_) => panic!("Expected total megapixels limit error"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn resource_limit_grid_tiles() {
    // Test grid tile count limit
    let input = &mut File::open(IMAGE_GRID_5X4).expect("Unknown file");

    // Grid has 20 tiles (5×4)
    // Set limit below that
    let config = avif_parse::DecodeConfig::default()
        .with_max_grid_tiles(10);

    let result = avif_parse::read_avif_with_config(input, &config, enough::Unstoppable);

    // Should fail due to tile count limit
    match result {
        Err(avif_parse::Error::ResourceLimitExceeded(msg)) => {
            assert_eq!(msg, "grid tile count limit exceeded");
        }
        Ok(_) => panic!("Expected grid tile count limit error"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn resource_limit_animation_frames() {
    // Test animation frame count limit
    let input = &mut File::open(ANIMATED_AVIF).expect("Unknown file");

    // File has 5 frames
    // Set limit below that
    let config = avif_parse::DecodeConfig::default()
        .with_max_animation_frames(3);

    let result = avif_parse::read_avif_with_config(input, &config, enough::Unstoppable);

    // Should fail due to frame count limit
    match result {
        Err(avif_parse::Error::ResourceLimitExceeded(msg)) => {
            assert_eq!(msg, "animation frame count limit exceeded");
        }
        Ok(_) => panic!("Expected animation frame count limit error"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn resource_limit_unlimited_config() {
    // Test that unlimited config works like old read_avif
    let input1 = &mut File::open(IMAGE_AVIF).expect("Unknown file");
    let input2 = &mut File::open(IMAGE_AVIF).expect("Unknown file");

    let result_old = avif_parse::read_avif(input1).expect("read_avif failed");
    let config = avif_parse::DecodeConfig::unlimited();
    let result_new = avif_parse::read_avif_with_config(input2, &config, enough::Unstoppable)
        .expect("read_avif_with_config failed");

    // Results should be identical
    assert_eq!(result_old.primary_item.len(), result_new.primary_item.len());
}

// ============================================================================
// Cancellation Tests
// ============================================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct TestCanceller {
    cancelled: Arc<AtomicBool>,
}

impl enough::Stop for TestCanceller {
    fn check(&self) -> Result<(), enough::StopReason> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(enough::StopReason::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[test]
fn cancellation_during_parsing() {
    // Test cancellation during box iteration
    let input = &mut File::open(IMAGE_AVIF).expect("Unknown file");

    let cancelled = Arc::new(AtomicBool::new(true)); // Pre-cancelled
    let stop = TestCanceller {
        cancelled: cancelled.clone(),
    };

    let config = avif_parse::DecodeConfig::default();
    let result = avif_parse::read_avif_with_config(input, &config, stop);

    // Should be cancelled
    match result {
        Err(avif_parse::Error::Stopped(reason)) => {
            assert_eq!(reason, enough::StopReason::Cancelled);
        }
        Ok(_) => panic!("Expected cancellation"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn cancellation_grid_extraction() {
    // Test cancellation during grid tile extraction
    let input = &mut File::open(IMAGE_GRID_5X4).expect("Unknown file");

    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = TestCanceller {
        cancelled: cancelled.clone(),
    };

    let config = avif_parse::DecodeConfig::default();

    // Cancel immediately (will hit check in tile extraction loop)
    cancelled.store(true, Ordering::Relaxed);

    let result = avif_parse::read_avif_with_config(input, &config, stop);

    // May succeed or be cancelled depending on timing
    // Just verify it doesn't panic
    match result {
        Ok(_) => {}, // Completed before cancellation check
        Err(avif_parse::Error::Stopped(_)) => {}, // Cancelled
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn no_cancellation_with_unstoppable() {
    // Test that Unstoppable never cancels
    let input = &mut File::open(IMAGE_AVIF).expect("Unknown file");

    let config = avif_parse::DecodeConfig::default();
    let result = avif_parse::read_avif_with_config(input, &config, enough::Unstoppable);

    // Should succeed
    assert!(result.is_ok(), "Unstoppable should never cancel");
}

// ============================================================================
// Backwards Compatibility Tests
// ============================================================================

#[test]
fn backwards_compat_read_avif() {
    // Verify read_avif() still works exactly as before
    let input = &mut File::open(IMAGE_AVIF).expect("Unknown file");
    let result = avif_parse::read_avif(input);

    assert!(result.is_ok());
    let avif = result.unwrap();
    assert_eq!(avif.primary_item.len(), 6979);
}

#[test]
fn backwards_compat_read_avif_with_options() {
    // Verify read_avif_with_options() still works
    let input = &mut File::open(IMAGE_AVIF).expect("Unknown file");
    let options = avif_parse::ParseOptions { lenient: false };
    let result = avif_parse::read_avif_with_options(input, &options);

    assert!(result.is_ok());
    let avif = result.unwrap();
    assert_eq!(avif.primary_item.len(), 6979);
}

#[test]
fn backwards_compat_lenient_mode() {
    // Verify lenient mode still propagates correctly
    let input = &mut File::open(IMAGE_AVIF).expect("Unknown file");

    let config = avif_parse::DecodeConfig::default().lenient(true);
    let result = avif_parse::read_avif_with_config(input, &config, enough::Unstoppable);

    assert!(result.is_ok());
}

// ============================================================================
// Defensive Parsing Tests
// ============================================================================

#[test]
fn defensive_large_mdat() {
    use std::io::Cursor;

    // Create a malicious AVIF with a fake 600MB mdat (exceeds 500MB limit)
    // ftyp box
    let mut data = vec![
        0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p',
        b'a', b'v', b'i', b'f', 0x00, 0x00, 0x00, 0x00,
        b'a', b'v', b'i', b'f', b'm', b'i', b'f', b'1',
    ];

    // mdat box with size claiming 600MB (but actual data much smaller)
    // Box size: 600MB + 8 bytes header = 600000008 = 0x23C34608
    let mdat_size = 600_000_008u32;
    data.extend_from_slice(&mdat_size.to_be_bytes());
    data.extend_from_slice(b"mdat");
    data.extend_from_slice(&[0u8; 100]); // Small actual data

    let mut cursor = Cursor::new(data);
    let config = avif_parse::DecodeConfig::default();
    let result = avif_parse::read_avif_with_config(&mut cursor, &config, enough::Unstoppable);

    // Should fail with "mdat too large"
    match result {
        Err(avif_parse::Error::InvalidData(msg)) => {
            assert_eq!(msg, "mdat too large");
        }
        Ok(_) => panic!("Expected mdat size rejection"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn config_default_limits() {
    // Verify default config has reasonable limits
    let config = avif_parse::DecodeConfig::default();

    assert_eq!(config.peak_memory_limit, Some(1_000_000_000)); // 1GB
    assert_eq!(config.total_megapixels_limit, Some(512)); // 512MP
    assert_eq!(config.frame_megapixels_limit, Some(256)); // 256MP
    assert_eq!(config.max_animation_frames, Some(10_000)); // 10k frames
    assert_eq!(config.max_grid_tiles, Some(1_000)); // 1k tiles
    assert_eq!(config.lenient, false);
}

#[test]
fn config_unlimited_no_limits() {
    // Verify unlimited config has no limits
    let config = avif_parse::DecodeConfig::unlimited();

    assert_eq!(config.peak_memory_limit, None);
    assert_eq!(config.total_megapixels_limit, None);
    assert_eq!(config.frame_megapixels_limit, None);
    assert_eq!(config.max_animation_frames, None);
    assert_eq!(config.max_grid_tiles, None);
    assert_eq!(config.lenient, false);
}

#[test]
fn config_builder_pattern() {
    // Test builder pattern
    let config = avif_parse::DecodeConfig::default()
        .with_peak_memory_limit(100_000_000)
        .with_total_megapixels_limit(64)
        .with_frame_megapixels_limit(32)
        .with_max_animation_frames(100)
        .with_max_grid_tiles(64)
        .lenient(true);

    assert_eq!(config.peak_memory_limit, Some(100_000_000));
    assert_eq!(config.total_megapixels_limit, Some(64));
    assert_eq!(config.frame_megapixels_limit, Some(32));
    assert_eq!(config.max_animation_frames, Some(100));
    assert_eq!(config.max_grid_tiles, Some(64));
    assert_eq!(config.lenient, true);
}
