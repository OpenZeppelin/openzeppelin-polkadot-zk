//! Tests for Confidential Assets Precompile
//!
//! These tests verify the ABI encoding/decoding helpers and selector constants.
//! Full integration tests require a mock runtime with pallet-revive configured.

use super::*;

mod helper_tests {
    use super::*;

    #[test]
    fn test_decode_u128_valid() {
        // u128 max value in 32-byte ABI encoding (right-aligned)
        let mut data = [0u8; 32];
        data[16..].copy_from_slice(&u128::MAX.to_be_bytes());

        let result = decode_u128(&data).unwrap();
        assert_eq!(result, u128::MAX);
    }

    #[test]
    fn test_decode_u128_zero() {
        let data = [0u8; 32];
        let result = decode_u128(&data).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_decode_u128_specific_value() {
        // Encode 1000 as u128 in ABI format
        let value: u128 = 1000;
        let mut data = [0u8; 32];
        data[16..].copy_from_slice(&value.to_be_bytes());

        let result = decode_u128(&data).unwrap();
        assert_eq!(result, 1000);
    }

    #[test]
    fn test_decode_u128_too_short() {
        let data = [0u8; 16]; // Only 16 bytes, need 32
        let result = decode_u128(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_u256_as_usize_valid() {
        // Offset of 64 (0x40)
        let mut data = [0u8; 32];
        data[31] = 64;

        let result = decode_u256_as_usize(&data).unwrap();
        assert_eq!(result, 64);
    }

    #[test]
    fn test_decode_u256_as_usize_zero() {
        let data = [0u8; 32];
        let result = decode_u256_as_usize(&data).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_decode_u256_as_usize_large_but_fits() {
        // A large but valid offset (1024)
        let value: u64 = 1024;
        let mut data = [0u8; 32];
        data[24..].copy_from_slice(&value.to_be_bytes());

        let result = decode_u256_as_usize(&data).unwrap();
        assert_eq!(result, 1024);
    }

    #[test]
    fn test_decode_dynamic_bytes_valid() {
        // Build ABI-encoded dynamic bytes
        // Structure: [offset (32)] [length (32)] [data...]
        let mut input = Vec::new();

        // Offset pointing to position 32 (right after offset itself)
        let offset: u64 = 32;
        let mut offset_bytes = [0u8; 32];
        offset_bytes[24..].copy_from_slice(&offset.to_be_bytes());
        input.extend_from_slice(&offset_bytes);

        // Length = 5
        let length: u64 = 5;
        let mut length_bytes = [0u8; 32];
        length_bytes[24..].copy_from_slice(&length.to_be_bytes());
        input.extend_from_slice(&length_bytes);

        // Actual data: "hello"
        input.extend_from_slice(b"hello");
        // Padding to 32-byte boundary
        input.extend_from_slice(&[0u8; 27]);

        let result = decode_dynamic_bytes(&input, 32).unwrap();
        assert_eq!(result, b"hello".to_vec());
    }

    #[test]
    fn test_decode_dynamic_bytes_empty() {
        let mut input = Vec::new();

        // Offset pointing to position 32
        let offset: u64 = 32;
        let mut offset_bytes = [0u8; 32];
        offset_bytes[24..].copy_from_slice(&offset.to_be_bytes());
        input.extend_from_slice(&offset_bytes);

        // Length = 0
        let length_bytes = [0u8; 32];
        input.extend_from_slice(&length_bytes);

        let result = decode_dynamic_bytes(&input, 32).unwrap();
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_decode_dynamic_bytes_invalid_offset() {
        let input = [0u8; 32]; // Only 32 bytes, offset would point outside
        let result = decode_dynamic_bytes(&input, 64);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_dynamic_bytes_length_overflow() {
        let mut input = Vec::new();

        // Offset pointing to position 32
        let offset: u64 = 32;
        let mut offset_bytes = [0u8; 32];
        offset_bytes[24..].copy_from_slice(&offset.to_be_bytes());
        input.extend_from_slice(&offset_bytes);

        // Length = 1000 (but we only have a few more bytes)
        let length: u64 = 1000;
        let mut length_bytes = [0u8; 32];
        length_bytes[24..].copy_from_slice(&length.to_be_bytes());
        input.extend_from_slice(&length_bytes);

        // Only 10 bytes of data (not enough for length=1000)
        input.extend_from_slice(&[0u8; 10]);

        let result = decode_dynamic_bytes(&input, 32);
        assert!(result.is_err());
    }
}

mod selector_tests {
    use super::selectors::*;

    #[test]
    fn test_selector_uniqueness() {
        // All selectors should be unique
        let all_selectors = [
            SET_PUBLIC_KEY,
            CONFIDENTIAL_TRANSFER,
            CONFIDENTIAL_BALANCE,
            DEPOSIT,
            WITHDRAW,
            CONFIDENTIAL_CLAIM,
            PUBLIC_KEY,
            TOTAL_SUPPLY,
        ];

        for (i, sel1) in all_selectors.iter().enumerate() {
            for (j, sel2) in all_selectors.iter().enumerate() {
                if i != j {
                    assert_ne!(sel1, sel2, "Selectors at {} and {} are identical", i, j);
                }
            }
        }
    }

    #[test]
    fn test_selector_non_zero() {
        // No selector should be all zeros
        let all_selectors = [
            SET_PUBLIC_KEY,
            CONFIDENTIAL_TRANSFER,
            CONFIDENTIAL_BALANCE,
            DEPOSIT,
            WITHDRAW,
            CONFIDENTIAL_CLAIM,
            PUBLIC_KEY,
            TOTAL_SUPPLY,
        ];

        for selector in all_selectors.iter() {
            assert_ne!(selector, &[0u8; 4], "Selector should not be zero");
        }
    }
}

mod abi_encoding_tests {
    use super::*;

    /// Helper to create ABI-encoded uint128
    fn abi_encode_u128(value: u128) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[16..].copy_from_slice(&value.to_be_bytes());
        result
    }

    /// Helper to create ABI-encoded address (32 bytes for AccountId32)
    fn abi_encode_address(account: [u8; 32]) -> [u8; 32] {
        account
    }

    #[test]
    fn test_abi_encode_u128_roundtrip() {
        let values = [0u128, 1, 100, 1000, u128::MAX / 2, u128::MAX];

        for value in values {
            let encoded = abi_encode_u128(value);
            let decoded = decode_u128(&encoded).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn test_confidential_balance_input_format() {
        // Test the expected input format for confidentialBalance(uint128, address)
        let asset_id: u128 = 42;
        let account: [u8; 32] = [1u8; 32];

        let mut input = Vec::new();
        input.extend_from_slice(&abi_encode_u128(asset_id));
        input.extend_from_slice(&abi_encode_address(account));

        assert_eq!(input.len(), 64);

        // Verify we can decode the asset_id
        let decoded_asset = decode_u128(&input[0..32]).unwrap();
        assert_eq!(decoded_asset, 42);

        // Verify the account bytes
        let decoded_account: [u8; 32] = input[32..64].try_into().unwrap();
        assert_eq!(decoded_account, account);
    }

    #[test]
    fn test_set_public_key_input_format() {
        // Test the expected input format for setPublicKey(bytes32)
        let public_key: [u8; 32] = [0xAB; 32];

        let input = public_key;

        assert_eq!(input.len(), 32);
        assert_eq!(input, public_key);
    }
}

mod precompile_address_tests {
    use super::*;
    use core::num::NonZero;

    #[test]
    fn test_precompile_address() {
        assert_eq!(PRECOMPILE_ADDRESS, 0x0C01);
    }

    #[test]
    fn test_precompile_address_is_nonzero() {
        // The address should be a valid NonZero<u16>
        let addr = NonZero::new(PRECOMPILE_ADDRESS);
        assert!(addr.is_some());
        assert_eq!(addr.unwrap().get(), 0x0C01);
    }

    #[test]
    fn test_precompile_address_bytes() {
        // The u16 0x0C01 should encode to bytes [0x0C, 0x01] in big endian
        // When placed in address bytes [16,17], the full address is:
        // 0x0000000000000000000000000000000C010000
        let addr_bytes = PRECOMPILE_ADDRESS.to_be_bytes();
        assert_eq!(addr_bytes, [0x0C, 0x01]);
    }
}

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_empty_input_returns_error() {
        let empty_input: &[u8] = &[];
        // decode_u128 should fail with empty input
        assert!(decode_u128(empty_input).is_err());
        // decode_u256_as_usize should fail with empty input
        assert!(decode_u256_as_usize(empty_input).is_err());
    }

    #[test]
    fn test_short_input_returns_error() {
        let short_input = [0u8; 10]; // Shorter than 32 bytes

        assert!(decode_u128(&short_input).is_err());
        assert!(decode_u256_as_usize(&short_input).is_err());
    }
}
