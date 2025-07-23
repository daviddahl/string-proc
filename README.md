# OTLP String Processing Proof-of-Concept

A Rust proof-of-concept demonstrating efficient string processing for OTLP (OpenTelemetry Protocol) data using dictionary-based deduplication and SIMD-accelerated UTF-8 validation.

## Overview

This project demonstrates a performance optimization technique for processing large volumes of string data from OTLP telemetry, where many strings are duplicated across different telemetry records (e.g., service names, attribute keys, etc.).

## Key Concepts

### The Problem
When processing OTLP data, the same strings appear repeatedly:
- Service names: `"user-service"`, `"payment-service"`
- Attribute keys: `"http.method"`, `"http.status_code"`, `"service.name"`
- Standard values: `"GET"`, `"POST"`, `"200"`, `"404"`

Traditional approaches validate each string individually, leading to redundant UTF-8 validation work.

### The Solution
This proof-of-concept implements a **dictionary-based deduplication** approach:

1. **Deduplication**: Collect unique byte slices into a dictionary
2. **Bulk Validation**: Validate each unique string only once using SIMD-accelerated UTF-8 validation
3. **Safe Conversion**: Convert validated bytes to strings without redundant validation
4. **Order Preservation**: Reconstruct the original order using dictionary indices

## Performance Benefits

- **Reduced UTF-8 Validation**: Each unique string is validated exactly once
- **SIMD Acceleration**: Uses `simdutf8` crate for faster validation than standard library
- **Memory Efficiency**: Stores each unique string only once during processing
- **Zero-Copy Potential**: Foundation for zero-copy string processing in Arrow/Parquet workflows

## Architecture

```
Raw Byte Slices → Dictionary → SIMD Validation → Safe Conversion → Result
     │                │             │                │              │
     │                │             │                │              │
  [b"service"]    {b"service": 0}    ✓ Valid      "service"     ["service", 
  [b"http"]       {b"http": 1}       ✓ Valid      "http"         "http",
  [b"service"]    └─ Duplicate       └─ Once       └─ Safe        "service"]
```

## Usage

### Basic Usage

```rust
use otlp_string_processing::process_otlp_strings;

let raw_data = vec![
    b"service.name".to_vec(),
    b"http.method".to_vec(),
    b"service.name".to_vec(),  // Duplicate
    b"region".to_vec(),
];

let processed = process_otlp_strings(raw_data)?;
// Result: ["service.name", "http.method", "service.name", "region"]
```

### Debug Mode

For detailed step-by-step processing information:

```rust
use otlp_string_processing::process_otlp_strings_debug;

let result = process_otlp_strings_debug(raw_data)?;
// Prints detailed logs of each processing step
```

## API Reference

### Functions

#### `process_otlp_strings(raw_strings: Vec<Vec<u8>>) -> Result<Vec<String>, OtlpProcessingError>`

Processes raw byte slices into validated UTF-8 strings with deduplication.

**Parameters:**
- `raw_strings`: Vector of byte vectors representing potential UTF-8 strings

**Returns:**
- `Ok(Vec<String>)`: Successfully processed strings in original order
- `Err(OtlpProcessingError)`: UTF-8 validation error

#### `process_otlp_strings_debug(raw_strings: Vec<Vec<u8>>) -> Result<Vec<String>, OtlpProcessingError>`

Same as `process_otlp_strings` but with detailed debug logging of each processing step.

### Types

#### `OtlpProcessingError`

Error type for processing failures:
- `Utf8Error(simdutf8::basic::Utf8Error)`: Invalid UTF-8 data encountered

## Examples

### Running the Example

```bash
cargo run
```

Output:
```
OTLP String Processing Example

Input: 7 raw byte strings (with duplicates)

Processed strings:
  0: service.name
  1: http.method
  2: service.name
  3: http.status_code
  4: region
  5: http.method
  6: trace.id

Total processed: 7 strings
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with debug output
cargo test test_debug_processing -- --nocapture

# Run invalid data test
cargo test test_debug_with_invalid_data -- --nocapture
```

## Test Coverage

The project includes comprehensive tests:

1. **`test_otlp_string_processing`**: Basic functionality test
2. **`test_invalid_data`**: Error handling for invalid UTF-8
3. **`test_debug_processing`**: Detailed step-by-step processing verification
4. **`test_debug_with_invalid_data`**: Debug mode error handling

### Sample Debug Output

```
=== OTLP String Processing Debug ===

Step 1: Building dictionary from 8 input strings
  Input[0]: [115, 101, 114, 118, 105, 99, 101, 46, 110, 97, 109, 101] (as string: 'service.name')
  Input[1]: [104, 116, 116, 112, 46, 109, 101, 116, 104, 111, 100] (as string: 'http.method')
  Input[2]: [115, 101, 114, 118, 105, 99, 101, 46, 110, 97, 109, 101] (as string: 'service.name')
  New entry: Input[0] -> Dictionary[0] ('service.name')
  New entry: Input[1] -> Dictionary[1] ('http.method')
  Found duplicate: Input[2] -> Dictionary[0] ('service.name')
  ...

Dictionary contents (5 unique entries):
  Dictionary[0]: [115, 101, 114, 118, 105, 99, 101, 46, 110, 97, 109, 101] -> 'service.name'
  Dictionary[1]: [104, 116, 116, 112, 46, 109, 101, 116, 104, 111, 100] -> 'http.method'
  ...

Step 2: Validating 5 unique dictionary entries using simdutf8
  ✓ Dictionary entry 0: 'service.name' is valid UTF-8
  ✓ Dictionary entry 1: 'http.method' is valid UTF-8
  ...
```

## Dependencies

- **`simdutf8`** (0.1.3): SIMD-accelerated UTF-8 validation
- **Standard Library**: HashMap, Vec for data structures

## Performance Characteristics

### Time Complexity
- **Dictionary Building**: O(n) where n is input size
- **Validation**: O(u) where u is unique strings (u ≤ n)
- **Result Construction**: O(n)
- **Overall**: O(n + u) vs O(n) for naive approach, with u << n in typical OTLP data

### Space Complexity
- **Dictionary**: O(u × average_string_length)
- **Index Array**: O(n)
- **Result**: O(n × average_string_length)
- **Total**: Similar to naive approach but with better cache locality

### Benchmarking Opportunities

This proof-of-concept provides a foundation for benchmarking:
- Dictionary approach vs. individual validation
- `simdutf8` vs. standard library UTF-8 validation
- Memory usage patterns with varying duplication rates
- Integration with Arrow/Parquet workflows

## Real-World Applications

### OTLP Processing Pipeline
```
OTLP Data → String Extraction → Dictionary Processing → Arrow Arrays → Parquet
```

### Telemetry Scenarios
- **High Duplication**: Service meshes with repeated service names
- **Medium Duplication**: HTTP attributes across multiple spans
- **Low Duplication**: Unique trace IDs and timestamps

## Future Enhancements

1. **Zero-Copy Integration**: Direct Arrow array construction
2. **Streaming Processing**: Handle data larger than memory
3. **Custom Allocators**: Optimize memory allocation patterns
4. **Parallel Processing**: Multi-threaded dictionary building
5. **Compression**: Dictionary-based string compression

## Development

### Project Structure
```
.
├── Cargo.toml          # Project configuration
├── README.md           # This file
└── src/
    ├── lib.rs          # Core implementation
    └── main.rs         # Example application
```

### Building
```bash
cargo build          # Debug build
cargo build --release # Optimized build
```

### Testing
```bash
cargo test                                    # All tests
cargo test -- --nocapture                   # With output
cargo test test_debug_processing -- --nocapture  # Specific test
```

## License

This is a proof-of-concept for educational and development purposes.

## Contributing

This project demonstrates core concepts for OTLP string processing optimization. Contributions welcome for:
- Performance improvements
- Additional test cases
- Integration examples
- Documentation enhancements
