use std::error;
use std::fmt;

use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Debug)]
pub enum HKDFError {
    UnrecognizedMessageVersion(u32),
}

impl fmt::Display for HKDFError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            HKDFError::UnrecognizedMessageVersion(message_version) => {
                write!(f, "unrecognized message version <{}>", message_version)
            }
        }
    }
}

impl error::Error for HKDFError {}

#[derive(Clone, Copy, Debug)]
pub struct HKDF {
    iteration_start_offset: u8,
}

impl HKDF {
    const HASH_OUTPUT_SIZE: usize = 32;

    pub fn new(message_version: u32) -> Result<Self, HKDFError> {
        match message_version {
            2 => Ok(HKDF {
                iteration_start_offset: 0,
            }),
            3 => Ok(HKDF {
                iteration_start_offset: 1,
            }),
            _ => Err(HKDFError::UnrecognizedMessageVersion(message_version)),
        }
    }

    pub fn derive_secrets(
        self,
        input_key_material: &[u8],
        info: &[u8],
        output_length: usize,
    ) -> Box<[u8]> {
        self.derive_salted_secrets(
            input_key_material,
            &[0u8; Self::HASH_OUTPUT_SIZE],
            info,
            output_length,
        )
    }

    pub fn derive_salted_secrets(
        self,
        input_key_material: &[u8],
        salt: &[u8],
        info: &[u8],
        output_length: usize,
    ) -> Box<[u8]> {
        let prk = self.extract(salt, input_key_material);
        self.expand(&prk, info, output_length)
    }

    fn extract(self, salt: &[u8], input_key_material: &[u8]) -> [u8; Self::HASH_OUTPUT_SIZE] {
        let mut mac =
            Hmac::<Sha256>::new_varkey(salt).expect("HMAC-SHA256 should accept any size key");
        mac.input(input_key_material);
        mac.result().code().into()
    }

    fn expand(
        self,
        prk: &[u8; Self::HASH_OUTPUT_SIZE],
        info: &[u8],
        output_length: usize,
    ) -> Box<[u8]> {
        let iterations = (output_length + Self::HASH_OUTPUT_SIZE - 1) / Self::HASH_OUTPUT_SIZE;
        let mut result = Vec::<u8>::with_capacity(iterations * Self::HASH_OUTPUT_SIZE);
        let mut mac =
            Hmac::<Sha256>::new_varkey(prk).expect("HMAC-SHA256 should accept any size key");

        for i in 0..iterations {
            if result.len() >= Self::HASH_OUTPUT_SIZE {
                mac.input(&result[(result.len() - Self::HASH_OUTPUT_SIZE)..]);
            }
            mac.input(info);
            mac.input(&[(i as u8) + self.iteration_start_offset]);
            let d = mac.result_reset().code();
            result.extend_from_slice(&d[..]);
        }

        result.truncate(output_length);
        result.into_boxed_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_v3() {
        let ikm = [
            0x0bu8, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
        ];
        let salt = [
            0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let info = [0xf0u8, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];
        let okm = [
            0x3cu8, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36,
            0x2f, 0x2a, 0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56,
            0xec, 0xc4, 0xc5, 0xbf, 0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65,
        ];

        let output = HKDF::new(3)
            .unwrap()
            .derive_salted_secrets(&ikm, &salt, &info, okm.len());

        assert_eq!(&okm[..], &output[..]);
    }

    #[test]
    fn test_vector_long_v3() {
        let ikm = [
            0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29,
            0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
            0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45,
            0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f,
        ];
        let salt = [
            0x60u8, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d,
            0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b,
            0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
            0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
            0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5,
            0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf,
        ];
        let info = [
            0xb0u8, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd,
            0xbe, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb,
            0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9,
            0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf, 0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
            0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5,
            0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
        ];
        let okm = [
            0xb1u8, 0x1e, 0x39, 0x8d, 0xc8, 0x03, 0x27, 0xa1, 0xc8, 0xe7, 0xf7, 0x8c, 0x59, 0x6a,
            0x49, 0x34, 0x4f, 0x01, 0x2e, 0xda, 0x2d, 0x4e, 0xfa, 0xd8, 0xa0, 0x50, 0xcc, 0x4c,
            0x19, 0xaf, 0xa9, 0x7c, 0x59, 0x04, 0x5a, 0x99, 0xca, 0xc7, 0x82, 0x72, 0x71, 0xcb,
            0x41, 0xc6, 0x5e, 0x59, 0x0e, 0x09, 0xda, 0x32, 0x75, 0x60, 0x0c, 0x2f, 0x09, 0xb8,
            0x36, 0x77, 0x93, 0xa9, 0xac, 0xa3, 0xdb, 0x71, 0xcc, 0x30, 0xc5, 0x81, 0x79, 0xec,
            0x3e, 0x87, 0xc1, 0x4c, 0x01, 0xd5, 0xc1, 0xf3, 0x43, 0x4f, 0x1d, 0x87,
        ];

        let output = HKDF::new(3)
            .unwrap()
            .derive_salted_secrets(&ikm, &salt, &info, okm.len());

        assert_eq!(&okm[..], &output[..]);
    }

    #[test]
    fn test_vector_v2() {
        let ikm = [
            0x0bu8, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
        ];
        let salt = [
            0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let info = [0xf0u8, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];
        let okm = [
            0x6eu8, 0xc2, 0x55, 0x6d, 0x5d, 0x7b, 0x1d, 0x81, 0xde, 0xe4, 0x22, 0x2a, 0xd7, 0x48,
            0x36, 0x95, 0xdd, 0xc9, 0x8f, 0x4f, 0x5f, 0xab, 0xc0, 0xe0, 0x20, 0x5d, 0xc2, 0xef,
            0x87, 0x52, 0xd4, 0x1e, 0x04, 0xe2, 0xe2, 0x11, 0x01, 0xc6, 0x8f, 0xf0, 0x93, 0x94,
            0xb8, 0xad, 0x0b, 0xdc, 0xb9, 0x60, 0x9c, 0xd4, 0xee, 0x82, 0xac, 0x13, 0x19, 0x9b,
            0x4a, 0xa9, 0xfd, 0xa8, 0x99, 0xda, 0xeb, 0xec,
        ];

        let output = HKDF::new(2)
            .unwrap()
            .derive_salted_secrets(&ikm, &salt, &info, okm.len());

        assert_eq!(&okm[..], &output[..]);
    }
}