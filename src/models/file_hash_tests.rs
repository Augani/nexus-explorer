use super::file_hash::*;
use proptest::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;







proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn property_hash_calculation_correctness(
        data in prop::collection::vec(any::<u8>(), 0..10000),
        algorithm in prop_oneof![
            Just(HashAlgorithm::Md5),
            Just(HashAlgorithm::Sha1),
            Just(HashAlgorithm::Sha256),
            Just(HashAlgorithm::Sha512),
        ]
    ) {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file.write_all(&data).expect("Failed to write data");
        temp_file.flush().expect("Failed to flush");
        
        let path = temp_file.path();
        
        let hash1 = calculate_file_hash(path, algorithm).expect("First hash calculation failed");
        let hash2 = calculate_file_hash(path, algorithm).expect("Second hash calculation failed");
        
        prop_assert_eq!(&hash1, &hash2, "Hash calculation should be deterministic");
        
        let expected_length = algorithm.hash_length();
        prop_assert_eq!(
            hash1.len(),
            expected_length,
            "Hash length for {:?} should be {} but got {}",
            algorithm,
            expected_length,
            hash1.len()
        );
        
        prop_assert!(
            is_valid_hex(&hash1),
            "Hash should be valid hexadecimal: {}",
            hash1
        );
    }
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn property_hash_bytes_matches_file_hash(
        data in prop::collection::vec(any::<u8>(), 0..5000),
        algorithm in prop_oneof![
            Just(HashAlgorithm::Md5),
            Just(HashAlgorithm::Sha1),
            Just(HashAlgorithm::Sha256),
            Just(HashAlgorithm::Sha512),
        ]
    ) {
        let hash_from_bytes = calculate_hash_bytes(&data, algorithm);
        
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file.write_all(&data).expect("Failed to write data");
        temp_file.flush().expect("Failed to flush");
        
        let hash_from_file = calculate_file_hash(temp_file.path(), algorithm)
            .expect("File hash calculation failed");
        
        prop_assert_eq!(
            hash_from_bytes,
            hash_from_file,
            "Hash from bytes should match hash from file"
        );
    }
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn property_algorithm_detection(
        data in prop::collection::vec(any::<u8>(), 1..1000),
        algorithm in prop_oneof![
            Just(HashAlgorithm::Md5),
            Just(HashAlgorithm::Sha1),
            Just(HashAlgorithm::Sha256),
            Just(HashAlgorithm::Sha512),
        ]
    ) {
        let hash = calculate_hash_bytes(&data, algorithm);
        let detected = detect_algorithm(&hash);
        
        prop_assert_eq!(
            detected,
            Some(algorithm),
            "Algorithm detection should identify {:?} from hash {}",
            algorithm,
            hash
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_known_md5_hash() {
        let hash = calculate_hash_bytes(&[], HashAlgorithm::Md5);
        assert_eq!(hash, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_known_sha1_hash() {
        let hash = calculate_hash_bytes(&[], HashAlgorithm::Sha1);
        assert_eq!(hash, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn test_known_sha256_hash() {
        let hash = calculate_hash_bytes(&[], HashAlgorithm::Sha256);
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_known_sha512_hash() {
        let hash = calculate_hash_bytes(&[], HashAlgorithm::Sha512);
        assert_eq!(
            hash,
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
    }

    #[test]
    fn test_hash_length() {
        assert_eq!(HashAlgorithm::Md5.hash_length(), 32);
        assert_eq!(HashAlgorithm::Sha1.hash_length(), 40);
        assert_eq!(HashAlgorithm::Sha256.hash_length(), 64);
        assert_eq!(HashAlgorithm::Sha512.hash_length(), 128);
    }

    #[test]
    fn test_is_valid_hex() {
        assert!(is_valid_hex("abc123"));
        assert!(is_valid_hex("ABC123"));
        assert!(is_valid_hex("0123456789abcdef"));
        assert!(!is_valid_hex(""));
        assert!(!is_valid_hex("xyz"));
        assert!(!is_valid_hex("abc 123"));
    }

    #[test]
    fn test_detect_algorithm() {
        assert_eq!(detect_algorithm("d41d8cd98f00b204e9800998ecf8427e"), Some(HashAlgorithm::Md5));
        assert_eq!(detect_algorithm("da39a3ee5e6b4b0d3255bfef95601890afd80709"), Some(HashAlgorithm::Sha1));
        assert_eq!(
            detect_algorithm("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            Some(HashAlgorithm::Sha256)
        );
        assert_eq!(detect_algorithm("abc"), None);
        assert_eq!(detect_algorithm("xyz"), None);
    }
}







proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn property_hash_comparison_accuracy(
        data in prop::collection::vec(any::<u8>(), 1..5000),
        algorithm in prop_oneof![
            Just(HashAlgorithm::Md5),
            Just(HashAlgorithm::Sha1),
            Just(HashAlgorithm::Sha256),
            Just(HashAlgorithm::Sha512),
        ]
    ) {
        let hash = calculate_hash_bytes(&data, algorithm);
        
        let result = compare_hashes(&hash, &hash);
        prop_assert_eq!(
            result,
            HashComparisonResult::Match,
            "Hash should match itself"
        );
        
        let upper_hash = hash.to_uppercase();
        let lower_hash = hash.to_lowercase();
        
        let result_upper = compare_hashes(&hash, &upper_hash);
        prop_assert_eq!(
            result_upper,
            HashComparisonResult::Match,
            "Hash comparison should be case-insensitive (upper)"
        );
        
        let result_lower = compare_hashes(&hash, &lower_hash);
        prop_assert_eq!(
            result_lower,
            HashComparisonResult::Match,
            "Hash comparison should be case-insensitive (lower)"
        );
        
        let result_mixed = compare_hashes(&upper_hash, &lower_hash);
        prop_assert_eq!(
            result_mixed,
            HashComparisonResult::Match,
            "Hash comparison should be case-insensitive (mixed)"
        );
    }
    
    #[test]
    fn property_different_hashes_mismatch(
        data1 in prop::collection::vec(any::<u8>(), 1..1000),
        data2 in prop::collection::vec(any::<u8>(), 1..1000),
        algorithm in prop_oneof![
            Just(HashAlgorithm::Md5),
            Just(HashAlgorithm::Sha1),
            Just(HashAlgorithm::Sha256),
            Just(HashAlgorithm::Sha512),
        ]
    ) {
        prop_assume!(data1 != data2);
        
        let hash1 = calculate_hash_bytes(&data1, algorithm);
        let hash2 = calculate_hash_bytes(&data2, algorithm);
        
        if hash1 != hash2 {
            let result = compare_hashes(&hash1, &hash2);
            prop_assert_eq!(
                result,
                HashComparisonResult::Mismatch,
                "Different hashes should not match"
            );
        }
    }
    
    #[test]
    fn property_invalid_hash_format(
        invalid_chars in "[^0-9a-fA-F]+",
    ) {
        prop_assume!(!invalid_chars.is_empty());
        
        let valid_hash = "d41d8cd98f00b204e9800998ecf8427e";
        
        let result = compare_hashes(valid_hash, &invalid_chars);
        prop_assert_eq!(
            result,
            HashComparisonResult::InvalidFormat,
            "Invalid hex should return InvalidFormat"
        );
    }
}

#[cfg(test)]
mod comparison_unit_tests {
    use super::*;

    #[test]
    fn test_compare_identical_hashes() {
        let hash = "d41d8cd98f00b204e9800998ecf8427e";
        assert_eq!(compare_hashes(hash, hash), HashComparisonResult::Match);
    }

    #[test]
    fn test_compare_case_insensitive() {
        let lower = "d41d8cd98f00b204e9800998ecf8427e";
        let upper = "D41D8CD98F00B204E9800998ECF8427E";
        assert_eq!(compare_hashes(lower, upper), HashComparisonResult::Match);
    }

    #[test]
    fn test_compare_different_hashes() {
        let hash1 = "d41d8cd98f00b204e9800998ecf8427e";
        let hash2 = "098f6bcd4621d373cade4e832627b4f6";
        assert_eq!(compare_hashes(hash1, hash2), HashComparisonResult::Mismatch);
    }

    #[test]
    fn test_compare_invalid_format() {
        let valid = "d41d8cd98f00b204e9800998ecf8427e";
        let invalid = "not-a-valid-hash";
        assert_eq!(compare_hashes(valid, invalid), HashComparisonResult::InvalidFormat);
    }

    #[test]
    fn test_compare_with_whitespace() {
        let hash1 = "  d41d8cd98f00b204e9800998ecf8427e  ";
        let hash2 = "d41d8cd98f00b204e9800998ecf8427e";
        assert_eq!(compare_hashes(hash1, hash2), HashComparisonResult::Match);
    }

    #[test]
    fn test_comparison_result_display() {
        assert_eq!(HashComparisonResult::Match.display_message(), "✓ Hashes match");
        assert_eq!(HashComparisonResult::Mismatch.display_message(), "✗ Hashes do not match");
        assert_eq!(HashComparisonResult::InvalidFormat.display_message(), "Invalid hash format");
    }
}
