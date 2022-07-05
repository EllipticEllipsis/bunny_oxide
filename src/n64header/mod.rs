pub mod ipl3;

use std::error::Error;

use byteorder::{BigEndian, ReadBytesExt};
use encoding_rs;
// use std::env;
// use std::fs;
use std::io;

#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Good,
    Bad,
    Ugly,
}

pub fn get_endian(input: &[u8]) -> Result<Endian, Box<dyn Error>> {
    match &input[0..4] {
        [0x80, 0x37, 0x12, 0x40] => Ok(Endian::Good),
        [0x40, 0x12, 0x37, 0x80] => Ok(Endian::Bad),
        [0x37, 0x80, 0x40, 0x12] => Ok(Endian::Ugly),
        _ => panic!("Unrecognised header format"),
    }
}

#[derive(Debug)]
pub struct N64Header {
    /* 0x00 */ pibsddomain1_register: [u8; 4],
    /* 0x04 */ clock_rate: u32,
    /* 0x08 */ entrypoint: u32,
    /* 0x0C */ revision: u32, /* Bottom byte is libultra version */
    /* 0x10 */ checksum1: u32,
    /* 0x14 */ checksum2: u32,
    /* 0x18 */ unk_18: [u8; 8],
    /* 0x20 */ image_name: [u8; 20], /* Internal ROM name */
    /* 0x34 */ unk_34: [u8; 4],
    /* 0x38 */ media_format: u32,
    /* 0x3C */ cartridge_id: [u8; 2],
    /* 0x3E */ country_code: u8,
    /* 0x3F */ version: u8,
}

impl N64Header {
    fn new(
        pibsddomain1_register: [u8; 4],
        clock_rate: u32,
        entrypoint: u32,
        revision: u32,
        checksum1: u32,
        checksum2: u32,
        unk_18: [u8; 8],
        image_name: [u8; 20],
        unk_34: [u8; 4],
        media_format: u32,
        cartridge_id: [u8; 2],
        country_code: u8,
        version: u8,
    ) -> N64Header {
        N64Header {
            pibsddomain1_register,
            clock_rate,
            entrypoint,
            revision,
            checksum1,
            checksum2,
            unk_18,
            image_name,
            unk_34,
            media_format,
            cartridge_id,
            country_code,
            version,
        }
    }

    pub fn entrypoint(&self) -> u32 {
        self.entrypoint
    }

    pub fn libultra_version(&self) -> Option<char> {
        char::from_u32(self.revision & 0xFF)
    }

    pub fn image_name(&self) -> String {
        encoding_rs::SHIFT_JIS
            .decode(&self.image_name)
            .0
            .to_string()
    }

    pub fn media_format(&self) -> char {
        char::from_u32(self.media_format)
            .expect(format!("could not parse media format \"{:X}\"", self.media_format).as_str())
    }

    pub fn cartridge_id(&self) -> String {
        String::from_utf8_lossy(&self.cartridge_id).to_string()
    }

    pub fn country_code(&self) -> char {
        char::from_u32(self.country_code as u32)
            .expect(format!("could not parse country code \"{:X}\"", self.media_format).as_str())
    }

    pub fn checksum(&self) -> (u32, u32) {
        (self.checksum1, self.checksum2)
    }

    pub fn media_format_description(&self) -> Result<&'static str, &'static str> {
        Ok(match self.media_format() {
            'N' => "cartridge",
            'D' => "64DD disk",
            'C' => "cartridge part of expandable game OR GameCube",
            'E' => "64DD expansion for cart",
            'Z' => "Aleck64 cartridge",
            _ => return Err("Unrecognised media format"),
        })
    }

    pub fn country_code_description(&self) -> Result<&'static str, &'static str> {
        Ok(match self.country_code() {
            '7' => "Beta",
            'A' => "Asian (NTSC)",
            'B' => "Brazilian",
            'C' => "Chinese",
            'D' => "German",
            'E' => "North America",
            'F' => "French",
            'G' => "Gateway 64 (NTSC)",
            'H' => "Dutch",
            'I' => "Italian",
            'J' => "Japanese",
            'K' => "Korean",
            'L' => "Gateway 64 (PAL)",
            'N' => "Canadian",
            'P' => "European (basic spec.)",
            'S' => "Spanish",
            'U' => "Australian",
            'W' => "Scandinavian",
            'X' => "European",
            'Y' => "European",
            '\0' => "iQue roms have zeros here",
            _ => return Err("Unrecognised country code"),
        })
    }
}

pub fn read_header(mut reader: impl io::Read) -> io::Result<N64Header> {
    let mut pibsddomain1_register = [0u8; 4];
    reader.read_exact(&mut pibsddomain1_register)?;

    let clock_rate = reader.read_u32::<BigEndian>()?;
    let entrypoint = reader.read_u32::<BigEndian>()?;
    let revision = reader.read_u32::<BigEndian>()?;
    let checksum1 = reader.read_u32::<BigEndian>()?;
    let checksum2 = reader.read_u32::<BigEndian>()?;
    let mut unk_18 = [0u8; 8];
    reader.read_exact(&mut unk_18)?;
    let mut image_name = [0u8; 20];
    reader.read_exact(&mut image_name)?;
    let mut unk_34 = [0u8; 4];
    reader.read_exact(&mut unk_34)?;
    let media_format = reader.read_u32::<BigEndian>()?;
    let mut cartridge_id = [0u8; 2];
    reader.read_exact(&mut cartridge_id)?;

    let country_code = reader.read_u8()?;
    let version = reader.read_u8()?;

    Ok(N64Header::new(
        pibsddomain1_register,
        clock_rate,
        entrypoint,
        revision,
        checksum1,
        checksum2,
        unk_18,
        image_name,
        unk_34,
        media_format,
        cartridge_id,
        country_code,
        version,
    ))
}

/// Use alternate format to get full version, normal for CSV of
/// clock_rate, entrypoint, revision, checksum, image_name, media_format, cartridge_id, country_code, version
impl std::fmt::Display for N64Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(
                f,
                "pibsddomain1_register:  {:02X} {:02X} {:02X} {:02X}\n\
            clock_rate:             {:08X}\n\
            reported_entrypoint:    {:08X}\n\
            revision:               {:08X}\n\
            checksum:               {:08X} {:08X}\n\
            unk_18:                 {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}\n\
            image_name:             \"{}\"\n\
            unk_34:                 {:02X} {:02X} {:02X} {:02X}\n\
            media_format:           {} ({})\n\
            cartridge_id:           {}\n\
            country_code:           {} ({})\n\
            version:                0x{:02X}",
                self.pibsddomain1_register[0],
                self.pibsddomain1_register[1],
                self.pibsddomain1_register[2],
                self.pibsddomain1_register[3],
                self.clock_rate,
                self.entrypoint,
                self.revision,
                self.checksum().0,
                self.checksum().1,
                self.unk_18[0],
                self.unk_18[1],
                self.unk_18[2],
                self.unk_18[3],
                self.unk_18[4],
                self.unk_18[5],
                self.unk_18[6],
                self.unk_18[7],
                self.image_name(),
                self.unk_34[0],
                self.unk_34[1],
                self.unk_34[2],
                self.unk_34[3],
                self.media_format(),
                self.media_format_description().unwrap(),
                self.cartridge_id(),
                self.country_code(),
                self.country_code_description().unwrap(),
                self.version,
            )
        } else {
            write!(
                f,
                "{:08X}, {:08X}, {:08X}, {:08X} {:08X}, {}, {}, {}, {}, {:X}",
                self.clock_rate,
                self.entrypoint,
                self.revision,
                self.checksum().0,
                self.checksum().1,
                self.image_name(),
                self.media_format(),
                self.cartridge_id(),
                self.country_code(),
                self.version
            )
        }
    }
}
