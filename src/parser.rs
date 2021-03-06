use crate::types::GroupData;
use crate::types::MemberAssignment;
use byteorder::{BigEndian, ReadBytesExt};
use chrono::prelude::*;
use chrono::Utc;
use rdkafka::message::Timestamp;
use std::io::{BufRead, Cursor};
use std::str;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn read_str<'a>(rdr: &'a mut Cursor<&[u8]>) -> Result<&'a str> {
    let len = (rdr.read_i16::<BigEndian>())? as usize;
    let pos = rdr.position() as usize;
    let slice = str::from_utf8(&rdr.get_ref()[pos..(pos + len)])?;
    rdr.consume(len);
    Ok(slice)
}

pub fn read_string(rdr: &mut Cursor<&[u8]>) -> Result<String> {
    read_str(rdr).map(str::to_string)
}

fn parse_group_offset(
    key_rdr: &mut Cursor<&[u8]>,
    payload_rdr: &mut Cursor<&[u8]>,
) -> Result<GroupData> {
    let group = read_string(key_rdr)?;
    let topic = read_string(key_rdr)?;
    let partition = key_rdr.read_i32::<BigEndian>()?;
    if !payload_rdr.get_ref().is_empty() {
        let _version = payload_rdr.read_i16::<BigEndian>()?;
        let offset = payload_rdr.read_i64::<BigEndian>()?;
        Ok(GroupData::OffsetCommit {
            group,
            topic,
            partition,
            offset,
        })
    } else {
        Ok(GroupData::None)
    }
}

pub fn parse_message(key: &[u8], payload: &[u8]) -> Result<GroupData> {
    let mut key_rdr = Cursor::new(key);
    let key_version = key_rdr.read_i16::<BigEndian>()?;
    match key_version {
        0 | 1 => parse_group_offset(&mut key_rdr, &mut Cursor::new(payload)),
        _ => panic!(),
    }
}

pub fn parse_date(timestamp: Timestamp) -> String {
    let t = timestamp.to_millis();
    let naive = NaiveDateTime::from_timestamp(t.unwrap() / 1000, 0);
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn parse_member_assignment(payload_rdr: &mut Cursor<&[u8]>) -> Result<Vec<MemberAssignment>> {
    let _version = payload_rdr.read_i16::<BigEndian>()?;
    let assign_len = payload_rdr.read_i32::<BigEndian>()?;
    let mut assigns = Vec::with_capacity(assign_len as usize);
    for _ in 0..assign_len {
        let topic = read_str(payload_rdr)?.to_owned();
        let partition_len = payload_rdr.read_i32::<BigEndian>()?;
        let mut partitions = Vec::with_capacity(partition_len as usize);
        for _ in 0..partition_len {
            let partition = payload_rdr.read_i32::<BigEndian>()?;
            partitions.push(partition);
        }
        assigns.push(MemberAssignment { topic, partitions })
    }
    Ok(assigns)
}
