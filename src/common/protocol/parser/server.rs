use crate::common::protocol::{CIPHERTEXT_SIZE, KEY_ID_SIZE, MSG_SIZE};
use anyhow::Context;

pub(crate) fn decode(
    data: &[u8; MSG_SIZE],
) -> anyhow::Result<(&[u8; KEY_ID_SIZE], &[u8; CIPHERTEXT_SIZE])> {
    let data_decoded = <&[u8; CIPHERTEXT_SIZE]>::try_from(&data[KEY_ID_SIZE..])
        .with_context(|| "Could not get decoded data for ciphertext")?;
    let key_id = <&[u8; KEY_ID_SIZE]>::try_from(&data[0..KEY_ID_SIZE])
        .with_context(|| "Could not get decoded data for key id")?;
    Ok((key_id, data_decoded))
}
