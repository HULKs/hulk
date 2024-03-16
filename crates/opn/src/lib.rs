use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
};

use byteorder::{BigEndian, ByteOrder};
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use sha2::{Digest, Sha256};

const OPN_MAGIC: [u8; 8] = *b"ALDIMAGE";

const HEADER_POSITION: u64 = 56;
const HEADER_SIZE: u64 = 4040;
const HEADER_CHECKSUM_POSITION: u64 = 24;

const INSTALLER_POSITION: u64 = 4096;
const INSTALLER_SIZE_POSITION: u64 = 96;
const INSTALLER_CHECKSUM_POSITION: u64 = 104;

const ROOTFS_CHECKSUM_POSITION: u64 = 136;

pub fn verify_image(image_path: impl AsRef<Path>) -> Result<()> {
    let mut file = File::open(&image_path)
        .wrap_err_with(|| format!("failed to open {}", image_path.as_ref().display()))?;

    let magic = read_exact_at(&mut file, 0, 8).wrap_err("failed to read magic from header")?;
    if magic != OPN_MAGIC {
        bail!(
            "magic doesn't match\nfound   : {}\nexpected: {}",
            String::from_utf8_lossy(&magic),
            String::from_utf8_lossy(&OPN_MAGIC)
        );
    }

    check_block(
        &mut file,
        HEADER_POSITION,
        HEADER_SIZE,
        HEADER_CHECKSUM_POSITION,
    )
    .wrap_err("header verification failed")?;

    let installer_size = read_u64(&mut file, INSTALLER_SIZE_POSITION)?;
    check_block(
        &mut file,
        INSTALLER_POSITION,
        installer_size,
        INSTALLER_CHECKSUM_POSITION,
    )
    .wrap_err("installer verification failed")?;

    let rootfs_position = INSTALLER_POSITION + installer_size;
    let image_size = file.metadata().wrap_err("failed to read metadata")?.len() - rootfs_position;
    check_block(
        &mut file,
        rootfs_position,
        image_size,
        ROOTFS_CHECKSUM_POSITION,
    )
    .wrap_err("rootfs verification failed")?;

    Ok(())
}

fn check_block(
    file: &mut File,
    data_position: u64,
    size: u64,
    checksum_position: u64,
) -> Result<()> {
    let checksum = read_u64(file, checksum_position).wrap_err("failed to read checksum")?;
    let data = read_exact_at(file, data_position, size).wrap_err("failed to read data block")?;

    verify_checksum(&data, checksum).wrap_err("checksum verification failed")
}

fn verify_checksum(data: &[u8], expected_checksum: u64) -> Result<()> {
    let calulated_checksum = Sha256::digest(data);
    let calulated_checksum = BigEndian::read_u64(&calulated_checksum);
    if calulated_checksum != expected_checksum {
        bail!("found   : {calulated_checksum}\nexpected: {expected_checksum}");
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
