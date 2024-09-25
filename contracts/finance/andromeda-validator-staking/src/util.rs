use cosmwasm_std::Binary;

pub fn decode_leb128(buf: &[u8]) -> u64 {
    let mut ret: u64 = 0;
    let mut offset = 0;

    while offset < buf.len() {
        let item: u8 = *buf.get(offset).unwrap_or(&0);
        ret += u64::from(item) << (7 * offset);

        if item < 0x80 {
            return ret;
        } else {
            ret -= 0x80 << (7 * offset);
        }
        offset += 1;
    }
    ret
}

pub fn decode_unstaking_response_data(data: Binary) -> (u64, u64) {
    /* extract seconds and nanoseconds from unstaking submessage reply data
        the unstaking reply data structure is as follow


        |--0 ~ 2--|-----------3 ~ 7----------|------|------------ 9~ --------------|
        | headers | seconds in leb128 format | 0x10 | nano second in leb128 format |
        Bytes 0 - 2 and 8 is used to identify proto tag and length, etc.
        Bytes 3 - 7 represent seconds in LEB128 format.
        Bytes 9 -  represent nano seconds in LEB128 format.

        Additional data can come after the nano second data depending on the cosmos sdk version used by the network. The decode algorithm will ignore additional data.

        Check unstaking response proto here for additional information(https://docs.cosmos.network/v0.46/modules/staking/03_messages.html)
    */
    let data = data.to_vec();
    let seconds = decode_leb128(&data[3..8]);

    let nano_seconds = decode_leb128(&data[9..]);
    (seconds, nano_seconds)
}

#[test]
fn test_decode_leb128() {
    let input = vec![0xd1, 0xfb, 0xc2, 0xb6, 0x06];
    let output = decode_leb128(&input);
    let expected_output = 1724956113;
    assert_eq!(output, expected_output);

    let input = vec![0xb8, 0xe3, 0xdd, 0xed, 0x02];
    let output = decode_leb128(&input);
    let expected_output = 766996920;
    assert_eq!(output, expected_output);

    let input = vec![0x9b, 0x96, 0xbd, 0xb6, 0x06];
    let output = decode_leb128(&input);
    let expected_output = 1724861211;
    assert_eq!(output, expected_output);

    let input = vec![0xda, 0xda, 0xa1, 0x4b];
    let output = decode_leb128(&input);
    let expected_output = 157838682;
    assert_eq!(output, expected_output);
}
#[test]
fn test_decode_unstaking_response_data() {
    let data = Binary::from(vec![
        0x0a, 0x0b, 0x08, 0x9b, 0x96, 0xbd, 0xb6, 0x06, 0x10, 0xda, 0xda, 0xa1, 0x4b,
    ]);
    let (sec, nsec) = decode_unstaking_response_data(data);

    let expected_output = (1724861211, 157838682);
    assert_eq!((sec, nsec), expected_output);

    let data = Binary::from(vec![
        0x0a, 0x0c, 0x08, 0xd1, 0xfb, 0xc2, 0xb6, 0x06, 0x10, 0xb8, 0xe3, 0xdd, 0xed, 0x02,
    ]);
    let (sec, nsec) = decode_unstaking_response_data(data);

    let expected_output = (1724956113, 766996920);
    assert_eq!((sec, nsec), expected_output);
}
