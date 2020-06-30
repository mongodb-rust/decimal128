use decimal128::*;

#[test]
fn it_returns_negative_infinity() {
    let vec: [u8; 16] = [
        0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let string = dec128.to_string();
    assert_eq!("-Infinity".to_string(), string);
}
#[test]
fn it_returns_positive_infinity() {
    let vec: [u8; 16] = [
        0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let string = dec128.to_string();
    assert_eq!("Infinity".to_string(), string);
}

#[test]
fn it_returns_nan() {
    let vec: [u8; 16] = [
        0x7c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let string = dec128.to_string();
    assert_eq!("NaN".to_string(), string);
}

#[test]
fn it_returns_0_001234() {
    let vec: [u8; 16] = [
        0x30, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
        0xd2,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let decimal = dec128.to_string();
    assert_eq!("0.001234".to_string(), decimal);
}

#[test]
fn it_returns_123456789012() {
    let vec: [u8; 16] = [
        0x30, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0xbe, 0x99, 0x1a,
        0x14,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let decimal = dec128.to_string();
    assert_eq!("123456789012".to_string(), decimal)
}

#[test]
fn it_returns_0_00123400000() {
    let vec: [u8; 16] = [
        0x30, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x5a, 0xef,
        0x40,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let decimal = dec128.to_string();
    assert_eq!("0.00123400000".to_string(), decimal)
}

#[test]
fn it_returns_0_1234567890123456789012345678901234() {
    let vec: [u8; 16] = [
        0x2f, 0xfc, 0x3c, 0xde, 0x6f, 0xff, 0x97, 0x32, 0xde, 0x82, 0x5c, 0xd0, 0x7e, 0x96, 0xaf,
        0xf2,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let decimal = dec128.to_string();
    assert_eq!("0.1234567890123456789012345678901234".to_string(), decimal)
}

#[test]
fn it_returns_1_000000000000000000000000000000000_e_6144() {
    let vec: [u8; 16] = [
        0x5f, 0xfe, 0x31, 0x4d, 0xc6, 0x44, 0x8d, 0x93, 0x38, 0xc1, 0x5b, 0x0a, 0x00, 0x00, 0x00,
        0x00,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let decimal = dec128.to_string();
    assert_eq!(
        "1.000000000000000000000000000000000E+6144".to_string(),
        decimal
    )
}

#[test]
fn it_returns_1_e_6176() {
    let vec: [u8; 16] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01,
    ];
    let dec128 = Decimal128::from_raw_bytes(vec);
    let decimal = dec128.to_string();
    assert_eq!("1E-6176".to_string(), decimal)
}
