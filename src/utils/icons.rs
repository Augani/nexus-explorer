/
/
/
/

/
/
/
/
#[inline]
pub fn rgba_to_bgra_pixel(r: u8, g: u8, b: u8, a: u8) -> (u8, u8, u8, u8) {
    (b, g, r, a)
}

/
/
/
/
pub fn rgba_to_bgra_inplace(data: &mut [u8]) {
    debug_assert!(
        data.len() % 4 == 0,
        "Data length must be a multiple of 4 (RGBA pixels)"
    );

    for chunk in data.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }
}

/
/
/
pub fn rgba_to_bgra(data: &[u8]) -> Vec<u8> {
    debug_assert!(
        data.len() % 4 == 0,
        "Data length must be a multiple of 4 (RGBA pixels)"
    );

    let mut result = data.to_vec();
    rgba_to_bgra_inplace(&mut result);
    result
}

/
/
/
pub fn bgra_to_rgba_inplace(data: &mut [u8]) {
    rgba_to_bgra_inplace(data);
}

/
pub fn bgra_to_rgba(data: &[u8]) -> Vec<u8> {
    rgba_to_bgra(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_to_bgra_pixel() {
        let (b, g, r, a) = rgba_to_bgra_pixel(255, 128, 64, 200);
        assert_eq!((b, g, r, a), (64, 128, 255, 200));
    }

    #[test]
    fn test_rgba_to_bgra_inplace() {
        let mut data = vec![255, 128, 64, 200, 100, 50, 25, 255];
        rgba_to_bgra_inplace(&mut data);
        assert_eq!(data, vec![64, 128, 255, 200, 25, 50, 100, 255]);
    }

    #[test]
    fn test_rgba_to_bgra() {
        let data = vec![255, 128, 64, 200];
        let result = rgba_to_bgra(&data);
        assert_eq!(result, vec![64, 128, 255, 200]);
    }

    #[test]
    fn test_round_trip() {
        let original = vec![255, 128, 64, 200, 100, 50, 25, 255];
        let bgra = rgba_to_bgra(&original);
        let back = bgra_to_rgba(&bgra);
        assert_eq!(original, back);
    }

    #[test]
    fn test_empty_data() {
        let mut data: Vec<u8> = vec![];
        rgba_to_bgra_inplace(&mut data);
        assert!(data.is_empty());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /
        /
        /
        /
        /
        #[test]
        fn prop_rgba_to_bgra_conversion(r in any::<u8>(), g in any::<u8>(), b in any::<u8>(), a in any::<u8>()) {
            let (out_b, out_g, out_r, out_a) = rgba_to_bgra_pixel(r, g, b, a);

            prop_assert_eq!(out_b, b, "Blue channel should be original blue");
            prop_assert_eq!(out_r, r, "Red channel should be original red");

            prop_assert_eq!(out_g, g, "Green channel should be preserved");
            prop_assert_eq!(out_a, a, "Alpha channel should be preserved");
        }

        /
        #[test]
        fn prop_rgba_bgra_round_trip(
            pixels in prop::collection::vec(any::<u8>(), 0..400)
                .prop_filter("Length must be multiple of 4", |v| v.len() % 4 == 0)
        ) {
            let bgra = rgba_to_bgra(&pixels);
            let back = bgra_to_rgba(&bgra);

            prop_assert_eq!(pixels, back, "Round trip should preserve original data");
        }
    }
}
