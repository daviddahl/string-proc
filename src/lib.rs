use std::collections::HashMap;
use simdutf8::basic::from_utf8; // For accelerated validation

/// Represents an error in validation or other processing steps.
#[derive(Debug)]
pub enum OtlpProcessingError {
    Utf8Error(simdutf8::basic::Utf8Error),
    // Add more variants if needed
}

impl From<simdutf8::basic::Utf8Error> for OtlpProcessingError {
    fn from(err: simdutf8::basic::Utf8Error) -> Self {
        OtlpProcessingError::Utf8Error(err)
    }
}

/// Processes a collection of raw byte slices (like OTLP-encoded strings).
/// 1. Collects and deduplicates byte slices (dictionary).
/// 2. Validates the unique entries using simdutf8 (once per unique string).
/// 3. Converts validated bytes to UTF-8 Strings without redundant copying.
pub fn process_otlp_strings(
    raw_strings: Vec<Vec<u8>>,
) -> Result<Vec<String>, OtlpProcessingError> {
    // Step 1: Build a dictionary to store each unique string once.
    //
    // We map each unique byte vector -> integer index.
    // The 'dictionary_array' holds the references (indexes) for each item in the
    // original input order.

    let mut dictionary: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut dictionary_array = Vec::with_capacity(raw_strings.len());

    for raw in raw_strings {
        if let Some(&existing_index) = dictionary.get(&raw) {
            // Already in dictionary
            dictionary_array.push(existing_index);
        } else {
            let new_index = dictionary.len();
            dictionary_array.push(new_index);
            dictionary.insert(raw, new_index);
        }
    }

    // Step 2: Validate all unique dictionary entries in bulk.
    //
    // We iterate over the dictionary keys (unique byte slices) and confirm
    // they are valid UTF-8 data using simdutf8. This ensures each unique
    // byte slice is validated exactly once.

    for key in dictionary.keys() {
        // If any entry is not valid UTF-8, this returns an error right away.
        from_utf8(key)?;
    }

    // Step 3: Convert the validated dictionary byte slices to final Strings.
    //
    // Because they've already been validated, we can safely use
    // `String::from_utf8_unchecked` to avoid a second pass of UTF-8 checks.
    // Note that this does allocate new Strings in memory. If the goal is truly
    // zero-copy, you'd need a more specialized data structure (e.g. Arrow arrays).
    //
    // For demonstration, we show a minimal-cost conversion:
    // - produce one String per unique entry

    let mut unique_strings = vec![String::new(); dictionary.len()];

    // dictionary is <Vec<u8>, usize>. We invert it here into the final Strings.
    for (key_bytes, index) in dictionary {
        let s = unsafe { String::from_utf8_unchecked(key_bytes) };
        unique_strings[index] = s;
    }

    // Step 4: Reconstruct full result (in the original order) using dictionary_array.
    // Each entry in dictionary_array references the unique validated String in unique_strings.

    let result: Vec<String> = dictionary_array
        .into_iter()
        .map(|idx| unique_strings[idx].clone())
        .collect();

    Ok(result)
}

/// Debug version of process_otlp_strings that prints detailed information about each step
pub fn process_otlp_strings_debug(
    raw_strings: Vec<Vec<u8>>,
) -> Result<Vec<String>, OtlpProcessingError> {
    println!("\n=== OTLP String Processing Debug ===\n");
    
    println!("Step 1: Building dictionary from {} input strings", raw_strings.len());
    for (i, bytes) in raw_strings.iter().enumerate() {
        println!("  Input[{}]: {:?} (as string: '{}')", 
            i, 
            bytes, 
            String::from_utf8_lossy(bytes)
        );
    }
    
    let mut dictionary: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut dictionary_array = Vec::with_capacity(raw_strings.len());

    for (input_idx, raw) in raw_strings.iter().enumerate() {
        if let Some(&existing_index) = dictionary.get(raw) {
            println!("  Found duplicate: Input[{}] -> Dictionary[{}] ('{}')", 
                input_idx, existing_index, String::from_utf8_lossy(raw));
            dictionary_array.push(existing_index);
        } else {
            let new_index = dictionary.len();
            println!("  New entry: Input[{}] -> Dictionary[{}] ('{}')", 
                input_idx, new_index, String::from_utf8_lossy(raw));
            dictionary_array.push(new_index);
            dictionary.insert(raw.clone(), new_index);
        }
    }
    
    println!("\nDictionary contents ({} unique entries):", dictionary.len());
    let mut dict_entries: Vec<_> = dictionary.iter().collect();
    dict_entries.sort_by_key(|(_, &index)| index);
    for (bytes, &index) in dict_entries {
        println!("  Dictionary[{}]: {:?} -> '{}'", 
            index, bytes, String::from_utf8_lossy(bytes));
    }
    
    println!("\nDictionary array (original order mapping): {:?}", dictionary_array);

    println!("\nStep 2: Validating {} unique dictionary entries using simdutf8", dictionary.len());
    for (i, key) in dictionary.keys().enumerate() {
        match from_utf8(key) {
            Ok(valid_str) => println!("  ✓ Dictionary entry {}: '{}' is valid UTF-8", i, valid_str),
            Err(e) => {
                println!("  ✗ Dictionary entry {}: {:?} is invalid UTF-8: {:?}", i, key, e);
                return Err(e.into());
            }
        }
    }

    println!("\nStep 3: Converting validated dictionary entries to Strings");
    let mut unique_strings = vec![String::new(); dictionary.len()];
    for (key_bytes, index) in dictionary {
        let s = unsafe { String::from_utf8_unchecked(key_bytes) };
        println!("  Dictionary[{}]: Converted to String '{}'", index, s);
        unique_strings[index] = s;
    }

    println!("\nStep 4: Reconstructing result in original order"); 
    let result: Vec<String> = dictionary_array
        .iter()
        .enumerate()
        .map(|(orig_idx, &dict_idx)| {
            let result_str = unique_strings[dict_idx].clone();
            println!("  Result[{}]: Dictionary[{}] -> '{}'", orig_idx, dict_idx, result_str);
            result_str
        })
        .collect();

    println!("\nFinal result: {:?}\n", result);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otlp_string_processing() {
        // Suppose these three byte slices arrived from OTLP:
        // - "service.name" repeated
        // - "status"
        // - "service.name" repeated again
        // - "region"
        let raw_data = vec![
            b"service.name".to_vec(),
            b"status".to_vec(),
            b"service.name".to_vec(),
            b"region".to_vec(),
        ];

        let processed = process_otlp_strings(raw_data).expect("UTF-8 validation failed");

        assert_eq!(processed, vec![
            "service.name".to_string(),
            "status".to_string(),
            "service.name".to_string(),
            "region".to_string(),
        ]);
    }

    #[test]
    fn test_invalid_data() {
        // Contains invalid UTF-8: 0xFF is not valid in UTF-8
        let invalid = vec![b"hello".to_vec(), vec![0xFF, 0xF0, 0x9F]];

        let result = process_otlp_strings(invalid);
        assert!(result.is_err(), "Should fail on invalid UTF-8 data");
    }

    #[test]
    fn test_debug_processing() {
        println!("\n🔍 Running debug test to show internal processing steps...");
        
        // Create a more interesting test case with multiple duplicates
        let raw_data = vec![
            b"service.name".to_vec(),      // New: Dictionary[0]
            b"http.method".to_vec(),       // New: Dictionary[1] 
            b"service.name".to_vec(),      // Duplicate of Dictionary[0]
            b"http.status_code".to_vec(),  // New: Dictionary[2]
            b"region".to_vec(),            // New: Dictionary[3]
            b"http.method".to_vec(),       // Duplicate of Dictionary[1]
            b"service.name".to_vec(),      // Another duplicate of Dictionary[0]
            b"trace.id".to_vec(),          // New: Dictionary[4]
        ];

        let processed = process_otlp_strings_debug(raw_data).expect("UTF-8 validation failed");

        // Verify the result maintains original order
        assert_eq!(processed, vec![
            "service.name".to_string(),
            "http.method".to_string(), 
            "service.name".to_string(),      // Duplicate
            "http.status_code".to_string(),
            "region".to_string(),
            "http.method".to_string(),       // Duplicate
            "service.name".to_string(),      // Another duplicate
            "trace.id".to_string(),
        ]);
        
        println!("✅ Debug test completed successfully!");
    }
    
    #[test] 
    fn test_debug_with_invalid_data() {
        println!("\n🔍 Testing debug function with invalid UTF-8 data...");
        
        let invalid_data = vec![
            b"valid_string".to_vec(),
            vec![0xFF, 0xFE, 0xFD], // Invalid UTF-8 bytes
            b"another_valid".to_vec(),
        ];
        
        let result = process_otlp_strings_debug(invalid_data);
        assert!(result.is_err(), "Should fail on invalid UTF-8 data");
        
        println!("✅ Invalid data test completed - correctly rejected invalid UTF-8!");
    }

    #[test]
    fn test_realistic_otlp_logs_string_extraction() {
        println!("\n📊 Testing realistic OTLP logs string extraction pattern...");
        
        // Simulate strings extracted from multiple OTLP LogRecord objects
        // This represents what would be extracted from the protobuf structures:
        
        let otlp_strings = vec![
            // --- Log Record 1: Web service request ---
            // Resource attributes
            b"service.name".to_vec(),           // Resource attribute key
            b"user-service".to_vec(),           // Resource attribute value
            b"service.version".to_vec(),        // Resource attribute key 
            b"1.2.3".to_vec(),                  // Resource attribute value
            b"deployment.environment".to_vec(), // Resource attribute key
            b"production".to_vec(),             // Resource attribute value
            
            // Instrumentation scope
            b"github.com/user-service/logger".to_vec(), // Scope name
            b"v0.1.0".to_vec(),                         // Scope version
            
            // Log record fields
            b"INFO".to_vec(),                   // severity_text
            b"http_request_completed".to_vec(), // event_name
            
            // Log attributes
            b"http.method".to_vec(),            // Log attribute key
            b"GET".to_vec(),                    // Log attribute value
            b"http.status_code".to_vec(),       // Log attribute key
            b"200".to_vec(),                    // Log attribute value
            b"http.route".to_vec(),             // Log attribute key
            b"/api/users/{id}".to_vec(),        // Log attribute value
            b"user.id".to_vec(),                // Log attribute key
            b"12345".to_vec(),                  // Log attribute value
            
            // Schema URL
            b"https://opentelemetry.io/schemas/1.21.0".to_vec(),
            
            // --- Log Record 2: Same service, different request (lots of duplicates) ---
            // Resource attributes (duplicates from Record 1)
            b"service.name".to_vec(),           // Duplicate
            b"user-service".to_vec(),           // Duplicate  
            b"service.version".to_vec(),        // Duplicate
            b"1.2.3".to_vec(),                  // Duplicate
            b"deployment.environment".to_vec(), // Duplicate
            b"production".to_vec(),             // Duplicate
            
            // Same instrumentation scope (duplicates)
            b"github.com/user-service/logger".to_vec(), // Duplicate
            b"v0.1.0".to_vec(),                         // Duplicate
            
            // Log record fields
            b"ERROR".to_vec(),                  // Different severity
            b"http_request_failed".to_vec(),    // Different event
            
            // Log attributes (mix of duplicates and new)
            b"http.method".to_vec(),            // Duplicate key
            b"POST".to_vec(),                   // Different value
            b"http.status_code".to_vec(),       // Duplicate key
            b"500".to_vec(),                    // Different value
            b"http.route".to_vec(),             // Duplicate key
            b"/api/orders".to_vec(),            // Different value
            b"error.type".to_vec(),             // New key
            b"DatabaseConnectionError".to_vec(), // New value
            b"error.message".to_vec(),          // New key
            b"Connection timeout after 30s".to_vec(), // New value
            
            // Same schema URL (duplicate)
            b"https://opentelemetry.io/schemas/1.21.0".to_vec(), // Duplicate
            
            // --- Log Record 3: Different service (payment-service) ---
            // Resource attributes
            b"service.name".to_vec(),           // Duplicate key, different value coming
            b"payment-service".to_vec(),        // New service name
            b"service.version".to_vec(),        // Duplicate key
            b"2.1.0".to_vec(),                  // Different version
            b"deployment.environment".to_vec(), // Duplicate key
            b"production".to_vec(),             // Duplicate value
            
            // Different instrumentation scope
            b"github.com/payment-service/tracer".to_vec(), // New scope
            b"v1.0.0".to_vec(),                            // Different version
            
            // Log record fields
            b"WARN".to_vec(),                   // Different severity
            b"payment_processing_slow".to_vec(), // New event
            
            // Log attributes
            b"payment.amount".to_vec(),         // New key
            b"99.99".to_vec(),                  // New value
            b"payment.currency".to_vec(),       // New key
            b"USD".to_vec(),                    // New value
            b"payment.method".to_vec(),         // New key
            b"credit_card".to_vec(),            // New value
            b"user.id".to_vec(),                // Duplicate key
            b"67890".to_vec(),                  // Different user ID
            
            // Same schema URL (duplicate)
            b"https://opentelemetry.io/schemas/1.21.0".to_vec(), // Duplicate
        ];
        
        println!("Simulating string extraction from {} OTLP log records", 3);
        println!("Total string fields extracted: {}", otlp_strings.len());
        
        // Process the strings using our dictionary approach
        let processed = process_otlp_strings_debug(otlp_strings.clone())
            .expect("All OTLP strings should be valid UTF-8");
        
        // Verify results
        assert_eq!(processed.len(), otlp_strings.len());
        
        // Verify some specific duplicates are preserved in order (based on debug output)
        assert_eq!(processed[0], "service.name");     // First occurrence
        assert_eq!(processed[19], "service.name");    // Second occurrence (duplicate)
        assert_eq!(processed[40], "service.name");    // Third occurrence (duplicate)
        
        assert_eq!(processed[1], "user-service");     // First service
        assert_eq!(processed[20], "user-service");    // Second occurrence (duplicate)
        assert_eq!(processed[41], "payment-service"); // Different service
        
        // Verify production environment appears multiple times
        assert_eq!(processed[5], "production");       // First occurrence
        assert_eq!(processed[24], "production");      // Second occurrence (duplicate)
        assert_eq!(processed[45], "production");      // Third occurrence (duplicate)
        
        // Verify schema URL appears multiple times
        assert_eq!(processed[18], "https://opentelemetry.io/schemas/1.21.0"); // First
        assert_eq!(processed[39], "https://opentelemetry.io/schemas/1.21.0"); // Second (duplicate)
        assert_eq!(processed[58], "https://opentelemetry.io/schemas/1.21.0"); // Third (duplicate)
        
        // Count unique vs total
        let mut unique_count = std::collections::HashSet::new();
        for s in &processed {
            unique_count.insert(s);
        }
        
        println!("\n📈 Statistics:");
        println!("  Total strings processed: {}", processed.len());
        println!("  Unique strings: {}", unique_count.len());
        println!("  Deduplication ratio: {:.1}%", 
            (processed.len() - unique_count.len()) as f64 / processed.len() as f64 * 100.0);
        println!("  Memory efficiency: {} validations avoided", 
            processed.len() - unique_count.len());
        
        // This demonstrates the key benefit: we validated only unique_count strings
        // instead of processed.len() strings, saving redundant UTF-8 validation work
        
        println!("✅ Realistic OTLP logs test completed successfully!");
    }
}
