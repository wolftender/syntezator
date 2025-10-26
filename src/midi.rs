use std::time::Duration;

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

    fn peek(&self) -> Option<u8> {
        if self.pointer < self.buffer.len() {
            self.buffer.get(self.pointer).copied()
        } else {
            None
        }
    }

    fn read_var_length(&mut self) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..4 {
            if let Some(byte) = self.read_u8() {
                if byte & 0x80 == 0 {
                    value = value + (byte as u32);
                    break;
                } else {
                    value = (value + ((byte & 0x7Fu8) as u32)) << 7;
                }
            } else {
                return None;
            }
        }

        Some(value)
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
    InvalidTrackChunk,
    InvalidEvent,
    InvalidTrackEventType,
    UnsupportedEvent,
    InvalidMetaEvent,
    UnexpectedMetaLength(u8, u32),
}

#[allow(clippy::upper_case_acronyms)]
pub enum SMPTE {
    _24,
    _25,
    _29_97,
    _30,
}

pub enum TimeDivision {
    TicksPerBit(u16),
    FramesPerSecond(SMPTE, u16),
}

impl TimeDivision {
    pub fn tick_duration(&self, tempo: Tempo) -> Duration {
        match self {
            TimeDivision::TicksPerBit(ticks) => {
                Duration::from_micros(tempo.as_mpqn() as u64) / (*ticks as u32)
            }
            TimeDivision::FramesPerSecond(smpte, ticks) => {
                let fps = match smpte {
                    SMPTE::_24 => 24.0,
                    SMPTE::_25 => 25.0,
                    SMPTE::_29_97 => 29.97,
                    SMPTE::_30 => 30.0,
                };

                Duration::from_secs(1) / (fps * (*ticks as f32)) as u32
            }
        }
    }
}

#[derive(Debug)]
pub enum ChannelEvent {
    NoteOff {
        delta_time: u32,
        channel: u8,
        note: u8,
        velocity: u8,
    },

    NoteOn {
        delta_time: u32,
        channel: u8,
        note: u8,
        velocity: u8,
    },

    NoteAftertouch {
        delta_time: u32,
        channel: u8,
        note: u8,
        aftertouch: u8,
    },

    Controller {
        delta_time: u32,
        channel: u8,
        controller_number: u8,
        controller_value: u8,
    },

    ProgramChange {
        delta_time: u32,
        channel: u8,
        program_number: u8,
        reserved: u8,
    },

    ChannelAftertouch {
        delta_time: u32,
        channel: u8,
        aftertouch: u8,
        reserved: u8,
    },

    PitchBend {
        delta_time: u32,
        channel: u8,
        lsb: u8,
        msb: u8,
    },
}

impl ChannelEvent {
    pub fn delta_time(&self) -> u32 {
        match self {
            Self::NoteOff { delta_time, .. } => *delta_time,
            Self::NoteOn { delta_time, .. } => *delta_time,
            Self::NoteAftertouch { delta_time, .. } => *delta_time,
            Self::Controller { delta_time, .. } => *delta_time,
            Self::ProgramChange { delta_time, .. } => *delta_time,
            Self::ChannelAftertouch { delta_time, .. } => *delta_time,
            Self::PitchBend { delta_time, .. } => *delta_time,
        }
    }

    pub fn channel(&self) -> u8 {
        match self {
            Self::NoteOff { channel, .. } => *channel,
            Self::NoteOn { channel, .. } => *channel,
            Self::NoteAftertouch { channel, .. } => *channel,
            Self::Controller { channel, .. } => *channel,
            Self::ProgramChange { channel, .. } => *channel,
            Self::ChannelAftertouch { channel, .. } => *channel,
            Self::PitchBend { channel, .. } => *channel,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tempo {
    /// Microseconds per quarter note
    mpqn: u32,
}

#[derive(Debug)]
pub enum MetaEvent {
    SequenceNumber {
        msb: u8,
        lsb: u8,
    },

    TextEvent {
        text: Vec<u8>,
    },

    CopyrightNotice {
        text: Vec<u8>,
    },

    SequenceTrackName {
        text: Vec<u8>,
    },

    InstrumentName {
        text: Vec<u8>,
    },

    Lyrics {
        text: Vec<u8>,
    },

    Marker {
        text: Vec<u8>,
    },

    CuePoint {
        text: Vec<u8>,
    },

    ChannelPrefix {
        channel: u8,
    },

    EndOfTrack,

    SetTempo {
        tempo: Tempo,
    },

    SMPTEOffset {
        hour: u8,
        min: u8,
        sec: u8,
        fs: u8,
        sub_fr: u8,
    },

    TimeSignature {
        number: u8,
        denom: u8,
        metro: u8,
        _32nds: u8,
    },

    KeySignature {
        key: i8,
        scale: u8,
    },

    UnknownEvent {
        event_type: u8,
        data: Vec<u8>,
    },

    SequencerSpecific {
        data: Vec<u8>,
    },
}

#[derive(Debug)]
pub enum MIDIEvent {
    Channel(ChannelEvent),
    Meta(MetaEvent),
}

impl Tempo {
    pub fn from_mpqn(mpqn: u32) -> Self {
        Self { mpqn }
    }

    pub fn from_bpm(bpm: u32) -> Self {
        const MICROSECONDS_PER_MINUTE: u32 = 60000000;
        let mpqn = MICROSECONDS_PER_MINUTE / bpm;

        Self { mpqn }
    }

    pub fn as_bpm(&self) -> u32 {
        const MICROSECONDS_PER_MINUTE: u32 = 60000000;
        MICROSECONDS_PER_MINUTE / self.mpqn
    }

    pub fn as_mpqn(&self) -> u32 {
        self.mpqn
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self::from_bpm(120)
    }
}

impl MIDIEvent {
    fn from_track_event(
        delta_time: u32,
        event_type: u8,
        channel: u8,
        param1: u8,
        param2: u8,
    ) -> Result<Self, MIDIFileError> {
        Ok(match event_type {
            0x8 => MIDIEvent::Channel(ChannelEvent::NoteOff {
                delta_time,
                channel,
                note: param1,
                velocity: param2,
            }),

            0x9 => MIDIEvent::Channel(ChannelEvent::NoteOn {
                delta_time,
                channel,
                note: param1,
                velocity: param2,
            }),

            0xA => MIDIEvent::Channel(ChannelEvent::NoteAftertouch {
                delta_time,
                channel,
                note: param1,
                aftertouch: param2,
            }),

            0xB => MIDIEvent::Channel(ChannelEvent::Controller {
                delta_time,
                channel,
                controller_number: param1,
                controller_value: param2,
            }),

            0xC => MIDIEvent::Channel(ChannelEvent::ProgramChange {
                delta_time,
                channel,
                program_number: param1,
                reserved: param2,
            }),

            0xD => MIDIEvent::Channel(ChannelEvent::ChannelAftertouch {
                delta_time,
                channel,
                aftertouch: param1,
                reserved: param2,
            }),

            0xE => MIDIEvent::Channel(ChannelEvent::PitchBend {
                delta_time,
                channel,
                lsb: param1,
                msb: param2,
            }),

            _ => return Err(MIDIFileError::InvalidTrackEventType),
        })
    }

    fn from_meta_event(event_reader: &mut BigEndianReader) -> Result<Self, MIDIFileError> {
        let event_type = event_reader
            .read_u8()
            .ok_or(MIDIFileError::InvalidMetaEvent)?;
        let event_length = event_reader
            .read_var_length()
            .ok_or(MIDIFileError::InvalidMetaEvent)?;

        match event_type {
            0x00 => {
                if event_length != 2 {
                    return Err(MIDIFileError::UnexpectedMetaLength(
                        event_type,
                        event_length,
                    ));
                }

                let msb = event_reader
                    .read_u8()
                    .ok_or(MIDIFileError::InvalidMetaEvent)?;
                let lsb = event_reader
                    .read_u8()
                    .ok_or(MIDIFileError::InvalidMetaEvent)?;

                Ok(MIDIEvent::Meta(MetaEvent::SequenceNumber { msb, lsb }))
            }

            0x01 => Ok(MIDIEvent::Meta(MetaEvent::TextEvent {
                text: Vec::from(
                    event_reader
                        .read_range(event_length as usize)
                        .ok_or(MIDIFileError::InvalidMetaEvent)?,
                ),
            })),

            0x02 => Ok(MIDIEvent::Meta(MetaEvent::CopyrightNotice {
                text: Vec::from(
                    event_reader
                        .read_range(event_length as usize)
                        .ok_or(MIDIFileError::InvalidMetaEvent)?,
                ),
            })),

            0x03 => Ok(MIDIEvent::Meta(MetaEvent::SequenceTrackName {
                text: Vec::from(
                    event_reader
                        .read_range(event_length as usize)
                        .ok_or(MIDIFileError::InvalidMetaEvent)?,
                ),
            })),

            0x04 => Ok(MIDIEvent::Meta(MetaEvent::InstrumentName {
                text: Vec::from(
                    event_reader
                        .read_range(event_length as usize)
                        .ok_or(MIDIFileError::InvalidMetaEvent)?,
                ),
            })),

            0x05 => Ok(MIDIEvent::Meta(MetaEvent::Lyrics {
                text: Vec::from(
                    event_reader
                        .read_range(event_length as usize)
                        .ok_or(MIDIFileError::InvalidMetaEvent)?,
                ),
            })),

            0x06 => Ok(MIDIEvent::Meta(MetaEvent::Marker {
                text: Vec::from(
                    event_reader
                        .read_range(event_length as usize)
                        .ok_or(MIDIFileError::InvalidMetaEvent)?,
                ),
            })),

            0x07 => Ok(MIDIEvent::Meta(MetaEvent::CuePoint {
                text: Vec::from(
                    event_reader
                        .read_range(event_length as usize)
                        .ok_or(MIDIFileError::InvalidMetaEvent)?,
                ),
            })),

            0x20 => {
                if event_length != 1 {
                    return Err(MIDIFileError::UnexpectedMetaLength(
                        event_type,
                        event_length,
                    ));
                }

                let channel = event_reader
                    .read_u8()
                    .ok_or(MIDIFileError::InvalidMetaEvent)?;

                Ok(MIDIEvent::Meta(MetaEvent::ChannelPrefix { channel }))
            }

            0x2F => {
                if event_length != 0 {
                    return Err(MIDIFileError::UnexpectedMetaLength(
                        event_type,
                        event_length,
                    ));
                }

                Ok(MIDIEvent::Meta(MetaEvent::EndOfTrack))
            }

            0x51 => {
                if event_length != 3 {
                    return Err(MIDIFileError::UnexpectedMetaLength(
                        event_type,
                        event_length,
                    ));
                }

                let bytes = event_reader
                    .read_range(3)
                    .ok_or(MIDIFileError::InvalidMetaEvent)?;

                let mpqn = (bytes[0] as u32) << 16 | (bytes[1] as u32) << 8 | (bytes[2] as u32);

                Ok(MIDIEvent::Meta(MetaEvent::SetTempo {
                    tempo: Tempo::from_mpqn(mpqn),
                }))
            }

            0x54 => {
                if event_length != 5 {
                    return Err(MIDIFileError::UnexpectedMetaLength(
                        event_type,
                        event_length,
                    ));
                }

                let bytes = event_reader
                    .read_range(5)
                    .ok_or(MIDIFileError::InvalidMetaEvent)?;

                Ok(MIDIEvent::Meta(MetaEvent::SMPTEOffset {
                    hour: bytes[0],
                    min: bytes[1],
                    sec: bytes[2],
                    fs: bytes[3],
                    sub_fr: bytes[4],
                }))
            }

            0x7F => {
                let bytes = event_reader
                    .read_range(event_length as usize)
                    .ok_or(MIDIFileError::InvalidMetaEvent)?;

                Ok(MIDIEvent::Meta(MetaEvent::SequencerSpecific {
                    data: Vec::from(bytes),
                }))
            }

            _ => {
                let bytes = event_reader
                    .read_range(event_length as usize)
                    .ok_or(MIDIFileError::InvalidMetaEvent)?;

                Ok(MIDIEvent::Meta(MetaEvent::UnknownEvent {
                    event_type,
                    data: Vec::from(bytes),
                }))
            }
        }
    }
}

pub struct MIDITrack {
    events: Vec<MIDIEvent>,
}

impl MIDITrack {
    pub fn events(&self) -> &[MIDIEvent] {
        &self.events
    }

    fn new(reader: &mut BigEndianReader) -> Result<MIDITrack, MIDIFileError> {
        if reader.read_range(4) != Some(MIDI_TRACK_CHUNK) {
            return Err(MIDIFileError::InvalidTrackChunk);
        }

        let chunk_size = reader.read_u32().ok_or(MIDIFileError::InvalidTrackChunk)?;
        let track_buffer = reader
            .read_range(chunk_size as usize)
            .ok_or(MIDIFileError::InvalidTrackChunk)?;

        let mut track_reader = BigEndianReader::new(track_buffer);
        let mut events = vec![];

        loop {
            let delta_time = track_reader
                .read_var_length()
                .ok_or(MIDIFileError::InvalidEvent)?;

            let type_byte = track_reader.read_u8().ok_or(MIDIFileError::InvalidEvent)?;

            match type_byte {
                0xFF => {
                    let event = MIDIEvent::from_meta_event(&mut track_reader)?;
                    let is_end_of_track = matches!(event, MIDIEvent::Meta(MetaEvent::EndOfTrack));

                    events.push(event);
                    if is_end_of_track {
                        break;
                    }
                }
                0xF0 => return Err(MIDIFileError::UnsupportedEvent),
                type_byte => {
                    let event_type = (0xf0u8 & type_byte) >> 4;
                    let channel = 0x0fu8 & type_byte;

                    let param1 = track_reader.read_u8().ok_or(MIDIFileError::InvalidEvent)?;
                    let param2 = track_reader.read_u8().ok_or(MIDIFileError::InvalidEvent)?;

                    let event = MIDIEvent::from_track_event(
                        delta_time, event_type, channel, param1, param2,
                    )?;

                    events.push(event);
                }
            }
        }

        Ok(Self { events })
    }
}

pub struct MIDIFileData {
    num_tracks: u16,
    tracks: Vec<MIDITrack>,
    time_division: TimeDivision,
}

impl MIDIFileData {
    pub fn num_tracks(&self) -> u16 {
        self.num_tracks
    }

    pub fn tracks(&self) -> &[MIDITrack] {
        &self.tracks
    }

    pub fn time_division(&self) -> &TimeDivision {
        &self.time_division
    }

    fn parse_time_division(value: u16) -> Result<TimeDivision, MIDIFileError> {
        if value & 0x8000u16 == 0 {
            Ok(TimeDivision::TicksPerBit(value & 0x7FFFu16))
        } else {
            let smpte_value = match (value & 0x7F00u16) >> 8 {
                24 => SMPTE::_24,
                25 => SMPTE::_25,
                29 => SMPTE::_29_97,
                30 => SMPTE::_30,
                _ => return Err(MIDIFileError::InvalidSMPTEValue),
            };

            let clock_ticks = value & 0x00FFu16;
            Ok(TimeDivision::FramesPerSecond(smpte_value, clock_ticks))
        }
    }
}

impl TryFrom<&[u8]> for MIDIFileData {
    type Error = MIDIFileError;

    fn try_from(buffer: &[u8]) -> Result<Self, MIDIFileError> {
        let mut reader = BigEndianReader::new(buffer);
        if reader.read_range(4) != Some(MIDI_HEADER_CHUNK) {
            return Err(MIDIFileError::HeaderMismatch);
        }

        if reader.read_u32() != Some(6u32) {
            return Err(MIDIFileError::HeaderSizeMismatch);
        }

        if reader.read_u16() != Some(0u16) {
            return Err(MIDIFileError::UnsupportedType);
        }

        let num_tracks = reader.read_u16().ok_or(MIDIFileError::InvalidTrackCount)?;
        let time_division = Self::parse_time_division(
            reader
                .read_u16()
                .ok_or(MIDIFileError::InvalidTimeDivision)?,
        )?;

        let mut tracks = vec![];
        for _ in 0..num_tracks {
            tracks.push(MIDITrack::new(&mut reader)?);
        }

        Ok(Self {
            tracks,
            num_tracks,
            time_division,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_var_len() {
        let test_vec1: Vec<u8> = vec![0b00000000];
        let test_vec2: Vec<u8> = vec![0b11001000];
        let test_vec3: Vec<u8> = vec![0b10000001, 0b01001000];
        let test_vec4: Vec<u8> = vec![0b11000000, 0b10000000, 0b00000000];

        fn read_int_from_buf_helper(buf: &[u8]) -> Option<u32> {
            let mut reader = BigEndianReader::new(buf);
            reader.read_var_length()
        }

        assert_eq!(read_int_from_buf_helper(&test_vec1), Some(0));
        assert_eq!(read_int_from_buf_helper(&test_vec2), None);
        assert_eq!(read_int_from_buf_helper(&test_vec3), Some(0xC8));
        assert_eq!(read_int_from_buf_helper(&test_vec4), Some(0x100000));
    }

    #[test]
    fn test_read_range() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xBBu8, 0xAAu8, 0x99u8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_range(3).unwrap(), &[0xFFu8, 0xEEu8, 0xDDu8]);
        assert_eq!(reader.read_range(2).unwrap(), &[0xCCu8, 0xBBu8]);
        assert!(reader.read_range(3).is_none());
        assert_eq!(reader.read_range(1).unwrap(), &[0xAAu8]);
        assert_eq!(reader.read_range(1).unwrap(), &[0x99u8]);
        assert!(reader.read_range(1).is_none());
    }

    #[test]
    fn test_read_u32() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xBBu8, 0xAAu8, 0x99u8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_u32().unwrap(), 0xFFEEDDCCu32);
        assert!(reader.read_u32().is_none());
        assert_eq!(reader.read_u16().unwrap(), 0xBBAAu16);
        assert!(reader.read_u32().is_none());
        assert!(reader.read_u16().is_none());
        assert_eq!(reader.read_u8().unwrap(), 0x99u8);
        assert!(reader.read_u32().is_none());
        assert!(reader.read_u32().is_none());
    }

    #[test]
    fn test_read_u16() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xBBu8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_u16().unwrap(), 0xFFEEu16);
        assert_eq!(reader.read_u16().unwrap(), 0xDDCCu16);
        assert!(reader.read_u16().is_none());
        assert_eq!(reader.read_u8().unwrap(), 0xBBu8);
        assert!(reader.read_u16().is_none());
    }

    #[test]
    fn test_read_u8() {
        let bytes = vec![0xFFu8, 0xEEu8, 0xDDu8];
        let mut reader = BigEndianReader::new(&bytes);

        assert_eq!(reader.read_u8().unwrap(), 0xFFu8);
        assert_eq!(reader.read_u8().unwrap(), 0xEEu8);
        assert_eq!(reader.read_u8().unwrap(), 0xDDu8);
        assert!(reader.read_u8().is_none());
    }

    #[test]
    fn test_midi_success() {
        let midi_bytes = include_bytes!("./assets/test.mid");
        let midi = MIDIFileData::try_from(&midi_bytes[..]).unwrap();

        let track = midi.tracks().first().unwrap();
        let last_event = track.events().last().unwrap();

        assert!(matches!(last_event, MIDIEvent::Meta(MetaEvent::EndOfTrack)))
    }
}
