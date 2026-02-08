# Streaming Parser Implementation TODO

## Completed ✓

1. **Error Types** (commit 4b86702)
   - Added `Error::ResourceLimitExceeded(&'static str)`
   - Added `Error::Stopped(enough::StopReason)`
   - Added `From<enough::StopReason>` impl
   - Added `enough = "0.3.1"` dependency

2. **DecodeConfig Structure** (commit 5a0fa3c)
   - Peak memory limit (default: 1GB)
   - Total megapixels limit (default: 512MP)
   - Frame megapixels limit (default: 256MP)
   - Max animation frames (default: 10,000)
   - Max grid tiles (default: 1,000)
   - Lenient mode flag
   - Builder pattern with `with_*` methods
   - `unlimited()` and `default()` constructors

3. **ResourceTracker** (commit 988ff07)
   - Internal struct for tracking allocations
   - `reserve/release` for peak memory tracking
   - `validate_collection_size` for pre-allocation checks
   - `validate_total_megapixels` for grid validation
   - `validate_frame_megapixels` for frame/tile validation
   - `validate_animation_frames` for frame count validation
   - `validate_grid_tiles` for tile count validation

## Remaining Work

### 4. Integrate ResourceTracker and Stop Trait into Parsing

**Goal**: Thread ResourceTracker and Stop through the main parsing flow.

**Files to modify**:
- `src/lib.rs`: Update `read_avif_with_options` to create new function

**New function**:
```rust
pub fn read_avif_with_config<T: Read>(
    f: &mut T,
    config: &DecodeConfig,
    stop: impl enough::Stop,
) -> Result<AvifData>
```

**Integration points**:

1. **Main box iteration** (around line 933 in `read_avif_with_options`):
   - Create `ResourceTracker::new(config)`
   - Add `stop.check()?` in the `while let Some(mut b) = iter.next_box()?` loop

2. **mdat allocation** (around line 950):
   - Validate mdat size before reading (max 500MB per mdat)
   - Call `tracker.reserve(size)` before allocation
   - Call `tracker.release(size)` after moving data into MediaDataBox

3. **Grid tile validation** (around line 1010):
   - Call `tracker.validate_grid_tiles(tiles_with_index.len() as u32)?`
   - Add `stop.check()?` every 16 tiles during extraction

4. **Grid dimension validation** (around line 1020):
   - Call `tracker.validate_total_megapixels(output_width, output_height)?`
   - Add overflow checks for grid calculation

5. **Animation frame validation** (around line 1077):
   - Get frame count from `sample_table.sample_to_chunk` length
   - Call `tracker.validate_animation_frames(frame_count)?`
   - Add `stop.check()?` every 16 frames in `extract_animation_frames`

**Helper function updates needed**:

1. **read_avif_meta** (line 1555):
   - Add `tracker: &ResourceTracker` parameter
   - Pass tracker to `read_iloc` for item count validation

2. **read_iloc** (line 2590):
   - Add `tracker: &ResourceTracker` parameter
   - Call `tracker.validate_collection_size::<ItemLocationBoxItem>(item_count)` before allocation
   - Validate item_count against file size (minimum bytes per item)

3. **read_ispe** (line 2176):
   - Add `tracker: &ResourceTracker` parameter
   - Call `tracker.validate_frame_megapixels(width, height)?`
   - Reject zero dimensions
   - Reject dimensions > 1M pixels per side

4. **read_moov** (line 2366):
   - Add `tracker: &ResourceTracker` parameter
   - Pass tracker down through `read_trak` → `read_mdia` → `read_minf` → `read_stbl`
   - Validate sample counts in `read_stsc`, `read_stsz`, `read_chunk_offsets`

5. **extract_animation_frames** (line 1892):
   - Add `stop: impl enough::Stop` parameter
   - Add `stop.check()?` every 16 frames in the extraction loop
   - Add `stop.check()?` every 16 chunks in sample table iteration

### 5. Add Backwards Compatibility Layer

**Goal**: Make existing `read_avif` and `read_avif_with_options` use new implementation.

**Changes**:
```rust
pub fn read_avif<T: Read>(f: &mut T) -> Result<AvifData> {
    read_avif_with_config(f, &DecodeConfig::unlimited(), enough::Unstoppable)
}

pub fn read_avif_with_options<T: Read>(f: &mut T, options: &ParseOptions) -> Result<AvifData> {
    let config = DecodeConfig::unlimited().lenient(options.lenient);
    read_avif_with_config(f, &config, enough::Unstoppable)
}
```

### 6. Add AvifParser Streaming Structure (Future Enhancement)

**Goal**: On-demand frame/tile extraction without eager loading.

**Not required for initial resource limits PR**, but documented in `STREAMING_ZEROCOPY_HANDOFF.md` for future work.

### 7. Add Tests

**File**: `tests/public.rs`

**Test categories**:

1. **Resource limit enforcement**:
   - Peak memory limit exceeded
   - Total megapixels limit exceeded
   - Frame megapixels limit exceeded
   - Animation frame count limit exceeded
   - Grid tile count limit exceeded

2. **Cancellation support**:
   - Cancel during box iteration
   - Cancel during grid extraction
   - Cancel during animation frame extraction

3. **Defensive parsing**:
   - File claiming 2^32 items (malicious iloc)
   - Extent larger than file size
   - Grid dimensions that overflow u64
   - Zero dimensions in ispe
   - Unrealistic frame counts (>10M)
   - Unrealistic tile counts (>1M)

4. **Backwards compatibility**:
   - Existing `read_avif()` calls work unchanged
   - Existing `read_avif_with_options()` calls work unchanged
   - Verify same results with unlimited config

## Testing Against Corpus

After implementation, test against:
- `link-u-samples/*.avifs` (animations)
- `av1-avif/testFiles/Microsoft/*.avif` (grid images)
- Verify no false positives with default limits

## Documentation Updates

- Update `README.md` with resource limit examples
- Document `DecodeConfig` usage patterns
- Migration guide for apps wanting to add limits
- Performance impact (should be negligible for valid files)

## Success Criteria

- [ ] All existing tests pass
- [ ] New resource limit tests pass (5+ scenarios)
- [ ] Cancellation tests pass (3+ scenarios)
- [ ] Defensive parsing tests pass (5+ malicious files)
- [ ] No unsafe code (except in c_api.rs)
- [ ] Backwards compatible (existing code works)
- [ ] Documentation complete
