const MIDI_HEADER_CHUNK: &[u8] = b"MThd";
const MIDI_TRACK_CHUNK: &[u8] = b"MTrk";

struct BigEndianReader<'a> {
    buffer: &'a [u8],
    pointer: usize,
}

impl<'a> BigEndianReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            pointer: 0usize,
        }
    }

    fn left_bytes(&self) -> usize {
        self.buffer.len() - self.pointer
    }

    fn read_n_bytes<F: Fn(&'a [u8]) -> R, R>(&mut self, n: usize, f: F) -> Option<R> {
        if self.left_bytes() >= n {
            let result = f(&self.buffer[self.pointer..self.pointer + n]);
            self.pointer = self.pointer + n;
            Some(result)
        } else {
            None
        }
    }

    fn read_u8(&mut self) -> Option<u8> {
        self.read_n_bytes(std::mem::size_of::<u8>(), |bytes| bytes[0])
    }

    fn read_u16(&mut self) -> Option<u16> {
        self.read_n_bytes(std::mem::size_of::<u16>(), |bytes| {
            (bytes[0] as u16) << 8 | (bytes[1] as u16)
        })
    }

    fn read_u32(&mut self) -> Option<u32> {
        self.read_n_bytes(std::mem::size_of::<u32>(), |bytes| {
            (bytes[0] as u32) << 24
                | (bytes[1] as u32) << 16
                | (bytes[2] as u32) << 8
                | (bytes[3] as u32)
        })
    }

    fn read_range(&mut self, n: usize) -> Option<&[u8]> {
        self.read_n_bytes(n, |bytes| bytes)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MIDIFileError {
    HeaderMismatch,
    HeaderSizeMismatch,
    UnsupportedType,
    InvalidTrackCount,
    InvalidTimeDivision,
    InvalidSMPTEValue,
}

pub enum SMPTE {
    _24,
    _25,
    _29,
    _30,
}

pub enum TimeDivision {
    TicksPerBit(u16),
    FramesPerSecond(SMPTE, u16),
}

pub struct MIDIFileData {
    num_tracks: u16,
    time_division: TimeDivision,
}

impl MIDIFileData {
    fn parse_time_division(value: u16) -> Result<TimeDivision, MIDIFileError> {
        if value & 0x8000u16 == 0 {
            return Ok(TimeDivision::TicksPerBit(value & 0x7FFFu16));
        } else {
            let smpte_value = match (value & 0x7F00u16) >> 8 {
                24 => SMPTE::_24,
                25 => SMPTE::_25,
                29 => SMPTE::_29,
                30 => SMPTE::_30,
                _ => return Err(MIDIFileError::InvalidSMPTEValue),
            };

            let clock_ticks = value & 0x00FFu16;
            return Ok(TimeDivision::FramesPerSecond(smpte_value, clock_ticks));
        }
    }

    pub fn from(buffer: &[u8]) -> Result<Self, MIDIFileError> {
        let mut reader = BigEndianReader::new(buffer);
        if reader.read_range(4) != Some(MIDI_HEADER_CHUNK) {
            return Err(MIDIFileError::HeaderMismatch);
        }

        if reader.read_u32() != Some(6u32) {
            return Err(MIDIFileError::HeaderSizeMismatch);
        }

        if reader.read_u32() != Some(1u32) {
            return Err(MIDIFileError::UnsupportedType);
        }

        let num_tracks = match reader.read_u16() {
            Some(value) => value,
            None => {
                return Err(MIDIFileError::InvalidTrackCount);
            }
        };

        let time_division = match reader.read_u16() {
            Some(value) => Self::parse_time_division(value),
            None => {
                return Err(MIDIFileError::InvalidTimeDivision);
            }
        }?;

        Ok(Self {
            num_tracks,
            time_division,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_range() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xBBu8, 0xAAu8, 0x99u8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_range(3).unwrap(), &[0xFFu8, 0xEEu8, 0xDDu8]);
        assert_eq!(reader.read_range(2).unwrap(), &[0xCCu8, 0xBBu8]);
        assert_eq!(reader.read_range(3).is_none(), true);
        assert_eq!(reader.read_range(1).unwrap(), &[0xAAu8]);
        assert_eq!(reader.read_range(1).unwrap(), &[0x99u8]);
        assert_eq!(reader.read_range(1).is_none(), true);
    }

    #[test]
    fn test_read_u32() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xBBu8, 0xAAu8, 0x99u8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_u32().unwrap(), 0xFFEEDDCCu32);
        assert_eq!(reader.read_u32().is_none(), true);
        assert_eq!(reader.read_u16().unwrap(), 0xBBAAu16);
        assert_eq!(reader.read_u32().is_none(), true);
        assert_eq!(reader.read_u16().is_none(), true);
        assert_eq!(reader.read_u8().unwrap(), 0x99u8);
        assert_eq!(reader.read_u32().is_none(), true);
        assert_eq!(reader.read_u32().is_none(), true);
    }

    #[test]
    fn test_read_u16() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xBBu8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_u16().unwrap(), 0xFFEEu16);
        assert_eq!(reader.read_u16().unwrap(), 0xDDCCu16);
        assert_eq!(reader.read_u16().is_none(), true);
        assert_eq!(reader.read_u8().unwrap(), 0xBBu8);
        assert_eq!(reader.read_u16().is_none(), true);
    }

    #[test]
    fn test_read_u8() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_u8().unwrap(), 0xFFu8);
        assert_eq!(reader.read_u8().unwrap(), 0xEEu8);
        assert_eq!(reader.read_u8().unwrap(), 0xDDu8);
        assert_eq!(reader.read_u8().is_none(), true);
    }

    #[test]
    fn test_midi_success() {
        let midi_bytes = include_bytes!("./assets/test.mid");
        let res = MIDIFileData::from(midi_bytes);

        assert_eq!(res.err().is_none(), true);
    }
}
