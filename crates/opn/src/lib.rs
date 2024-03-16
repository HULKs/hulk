use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
};

use byteorder::{BigEndian, ByteOrder};
use color_eyre::{
    eyre::{bail, Context, OptionExt},
    Result,
};
use sha2::{Digest, Sha256};

// #[repr(packed)]
// pub struct OpnHeader {
//     magic: [u8; 8],
//     flags: u8,
//     zero_a: [u8; 86],
//     unknown_a: u8,
//     installer_size_raw: u64,
//     zero_b: [u8; 64],
//     unknown_b: u8,
//     robot_kind: u8,
//     unknown_c: u8,
//     version: u64,
//     zero_c: [u8; 3896],
// }

pub fn verify_image(image_path: impl AsRef<Path>) -> Result<()> {
    let mut file = File::open(&image_path)
        .wrap_err_with(|| format!("failed to open {}", image_path.as_ref().display()))?;

    let mut buffer = [0; 8];
    file.read_exact(&mut buffer)
        .wrap_err("failed to read magic from header")?;
    println!("Magic: {}", String::from_utf8_lossy(&buffer));
    (buffer == b"ALDIMAGE"[..])
        .then_some(())
        .ok_or_eyre("magic doesn't match")?;

    let header_data = read_exact_at(&mut file, 56, 4040).wrap_err("failed to read header data")?;
    let header_checksum = read_u64(&mut file, 24).wrap_err("failed to read header checksum")?;
    verify_checksum(&header_data, header_checksum).wrap_err("header checksum does not match")?;

    let installer_size = read_u64(&mut file, 96)?;
    let installer_data =
        read_exact_at(&mut file, 4096, installer_size).wrap_err("failed to read intaller data")?;
    let installer_checksum =
        read_u64(&mut file, 104).wrap_err("failed to read installer checksum")?;
    verify_checksum(&installer_data, installer_checksum)
        .wrap_err("installer checksum does not match")?;

    let image_start = 4096 + installer_size;
    let image_size = file.metadata().wrap_err("failed to read metadata")?.len() - image_start;
    let image_data =
        read_exact_at(&mut file, image_start, image_size).wrap_err("failed to read image data")?;
    let image_checksum = read_u64(&mut file, 136).wrap_err("failed to read image checksum")?;
    verify_checksum(&image_data, image_checksum).wrap_err("image checksum does not match")?;

    Ok(())
}

fn verify_checksum(data: &[u8], expected_checksum: u64) -> Result<()> {
    let calulated_checksum = Sha256::digest(data);
    let calulated_checksum = BigEndian::read_u64(&calulated_checksum);
    if calulated_checksum != expected_checksum {
        bail!("expected: {expected_checksum}, actual: {calulated_checksum}");
    }

    Ok(())
}

fn read_exact_at(file: &mut File, data_position: u64, size: u64) -> Result<Vec<u8>> {
    let mut data = vec![0; size as usize];
    file.seek(std::io::SeekFrom::Start(data_position))?;
    file.read_exact(&mut data)?;

    Ok(data)
}

fn read_u64(file: &mut File, position: u64) -> Result<u64> {
    let mut buffer = [0; 8];
    file.seek(std::io::SeekFrom::Start(position))?;
    file.read_exact(&mut buffer)?;
    let size = BigEndian::read_u64(&buffer);

    Ok(size)
}
