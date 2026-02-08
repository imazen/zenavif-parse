# AVIF file structure parser (demuxer)

Get AV1 payload and the alpha channel metadata out of AVIF image files.

This is a minimal ISOBMFF/MIAF and AV1 OBU parser. It is a fork of Mozilla's MP4 parser used in Firefox, so it's designed to be robust and safely handle untrusted data. This crate is written entirely in safe Rust code.

The parser is compatible with files supported by libavif, Chrome 85 and Firefox 81a. It parses all files in [the AOM test suite](https://github.com/AOMediaCodec/av1-avif).

[API documentation](https://docs.rs/avif-parse/)

This crate doesn't include an AV1 decoder. To display the pixels you will additionally need [dav1d](https://code.videolan.org/videolan/dav1d) or [libaom](https://lib.rs/libaom-sys) ([full decoder example](https://gitlab.com/kornelski/aom-decode)).

## Usage from Rust

It takes `io::Read`, so you can use any readable input, such as a byte slice (`vec.as_slice()`), or a `File` wrapped in `BufReader`, etc.

```rust
let data = read_avif(&mut slice)?;
av1_decode(&data.primary_item)?;
if let Some(alpha) = &data.alpha_item {
    av1_decode(alpha)?;
}
if data.premultiplied_alpha {
    // after decoding, remember to divide R,G,B values by A
}
```

### Resource Limits and Defensive Parsing

For untrusted input, use `read_avif_with_config()` to enforce resource limits:

```rust
use avif_parse::{read_avif_with_config, DecodeConfig};

// Conservative limits for untrusted files
let config = DecodeConfig::default()
    .with_peak_memory_limit(100_000_000)  // 100MB max
    .with_total_megapixels_limit(64)       // 64MP max
    .with_max_animation_frames(100);       // 100 frames max

let data = read_avif_with_config(&mut file, &config, enough::Unstoppable)?;
```

**Default limits** (applied with `DecodeConfig::default()`):
- Peak memory: 1GB
- Total megapixels: 512MP
- Frame megapixels: 256MP
- Max animation frames: 10,000
- Max grid tiles: 1,000

**Unlimited parsing** (backwards compatible):
```rust
let config = DecodeConfig::unlimited();
let data = read_avif_with_config(&mut file, &config, enough::Unstoppable)?;
```

### Cooperative Cancellation

Long-running parsing can be cancelled using the `Stop` trait:

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct Canceller {
    cancelled: Arc<AtomicBool>,
}

impl enough::Stop for Canceller {
    fn check(&self) -> Result<(), enough::StopReason> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(enough::StopReason::Cancelled)
        } else {
            Ok(())
        }
    }
}

let cancelled = Arc::new(AtomicBool::new(false));
let stop = Canceller { cancelled: cancelled.clone() };

// In another thread: cancelled.store(true, Ordering::Relaxed);

match read_avif_with_config(&mut file, &config, stop) {
    Ok(data) => { /* completed */ },
    Err(Error::Stopped(_)) => { /* cancelled */ },
    Err(e) => { /* error */ },
}
```

## Usage from C

Install Rust 1.68 or later, preferably via [rustup](https://rustup.rs), and run:

```bash
cargo build --release
```

It will build `./target/release/libavif_parse.a` (or `avif_parse.lib` on Windows). Link it with your project.

Cargo supports cross-compilation, so you can easily build it for other platforms (e.g. [iOS](https://lib.rs/crates/cargo-xcode)).

```c
#include "avif_parse.h"
avif_data_t data = avif_parse(file_data, file_length);

if (data) {
    av1_decode(data.primary_data, data.primary_size);
    if (data.alpha_data) {
        av1_decode(data.alpha_data, data.alpha_size);
    }
    if (data.premultiplied_alpha) {
        // after decoding, remember to divide R,G,B values by A
    }
}

avif_data_free(data);
```
