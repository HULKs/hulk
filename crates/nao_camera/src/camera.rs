use std::{
    ffi::{CString, NulError},
    io,
    os::unix::prelude::OsStrExt,
    path::Path,
    time::Duration,
};

use libc::{close, open, poll, pollfd, O_NONBLOCK, O_RDWR, POLLIN, POLLPRI};
use nix::errno::Errno;
use thiserror::Error;

use crate::{
    automatic_exposure_control_weights::{
        set_automatic_exposure_control_weights, ExposureWeightsError,
    },
    bindings::{
        V4L2_CID_AUTO_WHITE_BALANCE, V4L2_CID_BRIGHTNESS, V4L2_CID_CONTRAST,
        V4L2_CID_EXPOSURE_ABSOLUTE, V4L2_CID_EXPOSURE_AUTO, V4L2_CID_FOCUS_ABSOLUTE,
        V4L2_CID_FOCUS_AUTO, V4L2_CID_GAIN, V4L2_CID_HUE, V4L2_CID_HUE_AUTO, V4L2_CID_SATURATION,
        V4L2_CID_SHARPNESS, V4L2_CID_WHITE_BALANCE_TEMPERATURE,
    },
    controls::{set_control, SetControlError},
    digital_effects::{disable_digital_effects, DigitalEffectsError},
    flip::{flip_sensor, FlipError},
    format::{set_format, SetFormatError},
    parameters::{CameraParameters, ExposureMode, Format},
    queueing::{dequeue, queue, QueueingError},
    request_buffers::{request_user_pointer_buffers, RequestBuffersError},
    streaming::{stream_off, stream_on, StreamingError},
    time_per_frame::{set_time_per_frame, SetTimePerFrameError},
};

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("failed to convert path")]
    PathNotConverted { source: NulError },
    #[error("failed to open device")]
    DeviceNotOpen { source: io::Error },
    #[error("failed set format to {width}x{height} with {format:?}")]
    FormatNotSet {
        source: SetFormatError,
        width: u32,
        height: u32,
        format: Format,
    },
    #[error("failed set time per frame to {numerator}/{denominator}")]
    TimePerFrameNotSet {
        source: SetTimePerFrameError,
        numerator: u32,
        denominator: u32,
    },
    #[error("failed to set brightness to {brightness}")]
    BrightnessNotSet {
        source: SetControlError,
        brightness: i32,
    },
    #[error("failed to set contrast to {contrast}")]
    ContrastNotSet {
        source: SetControlError,
        contrast: i32,
    },
    #[error("failed to set saturation to {saturation}")]
    SaturationNotSet {
        source: SetControlError,
        saturation: i32,
    },
    #[error("failed to set hue to {hue}")]
    HueNotSet { source: SetControlError, hue: i32 },
    #[error("failed to set white_balance_temperature_auto to {white_balance_temperature_auto}")]
    WhiteBalanceTemperatureAutoNotSet {
        source: SetControlError,
        white_balance_temperature_auto: bool,
    },
    #[error("failed to set gain to {gain}")]
    GainNotSet { source: SetControlError, gain: i32 },
    #[error("failed to set hue_auto to {hue_auto}")]
    HueAutoNotSet {
        source: SetControlError,
        hue_auto: bool,
    },
    #[error("failed to set white_balance_temperature to {white_balance_temperature}")]
    WhiteBalanceTemperatureNotSet {
        source: SetControlError,
        white_balance_temperature: i32,
    },
    #[error("failed to set sharpness to {sharpness}")]
    SharpnessNotSet {
        source: SetControlError,
        sharpness: i32,
    },
    #[error("failed to set exposure_auto to {exposure_auto:?}")]
    ExposureAutoNotSet {
        source: SetControlError,
        exposure_auto: ExposureMode,
    },
    #[error("failed to set exposure_absolute to {exposure_absolute}")]
    ExposureAbsoluteNotSet {
        source: SetControlError,
        exposure_absolute: i32,
    },
    #[error("failed to set focus_absolute to {focus_absolute}")]
    FocusAbsoluteNotSet {
        source: SetControlError,
        focus_absolute: i32,
    },
    #[error("failed to set focus_auto to {focus_auto}")]
    FocusAutoNotSet {
        source: SetControlError,
        focus_auto: bool,
    },
    #[error("failed to set automatic exposure control weights to {weights:?}")]
    AutomaticExposureControlWeightsNotSet {
        source: ExposureWeightsError,
        weights: [u8; 16],
    },
    #[error("failed to flip camera sensor")]
    NotFlipped { source: FlipError },
    #[error("failed to disable digital effects")]
    DigitalEffectsNotDisabled { source: DigitalEffectsError },
    #[error("failed to request {amount_of_buffers} user-pointer buffers")]
    UserPointerBuffersNotRequested {
        source: RequestBuffersError,
        amount_of_buffers: u32,
    },
}

#[derive(Debug, Error)]
pub enum PollingError {
    #[error("failed to poll device")]
    DeviceNotPolled { source: Errno },
    #[error("polling device timed out")]
    DevicePollingTimedOut,
    #[error("failed to poll device: returned events ({returned_events}) does not contain POLLIN | POLLPRI")]
    DevicePollingReturnedNoEvents { returned_events: i16 },
}

#[derive(Debug, Error)]
pub enum BufferError {
    #[error("no free slot available to queue")]
    NoSlotAvailable { non_fitting_buffer: Vec<u8> },
    #[error("failed to queue buffer")]
    BufferNotQueued { source: QueueingError },
    #[error("failed to dequeue buffer")]
    BufferNotDequeued { source: QueueingError },
    #[error("v4l2 returned buffer index {buffer_index} where no slot is allocated")]
    SlotNotOccupied { buffer_index: u32 },
}

pub struct Camera {
    file_descriptor: i32,
    queued_buffers: Vec<Option<Vec<u8>>>,
    next_queued_buffer_index: usize,
}

impl Camera {
    pub fn open(path: impl AsRef<Path>, parameters: &CameraParameters) -> Result<Self, OpenError> {
        let path = CString::new(path.as_ref().as_os_str().as_bytes())
            .map_err(|source| OpenError::PathNotConverted { source })?;
        let file_descriptor = unsafe { open(path.as_ptr(), O_RDWR | O_NONBLOCK) };
        if file_descriptor == -1 {
            return Err(OpenError::DeviceNotOpen {
                source: io::Error::last_os_error(),
            });
        }

        set_format(
            file_descriptor,
            parameters.width,
            parameters.height,
            match parameters.format {
                Format::YUVU => {
                    'Y' as u32 | ('U' as u32) << 8 | ('Y' as u32) << 16 | ('V' as u32) << 24
                }
            },
        )
        .map_err(|source| OpenError::FormatNotSet {
            source,
            width: parameters.width,
            height: parameters.height,
            format: parameters.format,
        })?;

        set_time_per_frame(
            file_descriptor,
            parameters.time_per_frame.numerator,
            parameters.time_per_frame.denominator,
        )
        .map_err(|source| OpenError::TimePerFrameNotSet {
            source,
            numerator: parameters.time_per_frame.numerator,
            denominator: parameters.time_per_frame.denominator,
        })?;

        set_control(file_descriptor, V4L2_CID_BRIGHTNESS, parameters.brightness).map_err(
            |source| OpenError::BrightnessNotSet {
                source,
                brightness: parameters.brightness,
            },
        )?;
        set_control(file_descriptor, V4L2_CID_CONTRAST, parameters.contrast).map_err(|source| {
            OpenError::ContrastNotSet {
                source,
                contrast: parameters.contrast,
            }
        })?;
        set_control(file_descriptor, V4L2_CID_SATURATION, parameters.saturation).map_err(
            |source| OpenError::SaturationNotSet {
                source,
                saturation: parameters.saturation,
            },
        )?;
        set_control(file_descriptor, V4L2_CID_HUE, parameters.hue).map_err(|source| {
            OpenError::HueNotSet {
                source,
                hue: parameters.hue,
            }
        })?;
        set_control(
            file_descriptor,
            V4L2_CID_AUTO_WHITE_BALANCE,
            match parameters.white_balance_temperature_auto {
                true => 1,
                false => 0,
            },
        )
        .map_err(|source| OpenError::WhiteBalanceTemperatureAutoNotSet {
            source,
            white_balance_temperature_auto: parameters.white_balance_temperature_auto,
        })?;
        set_control(file_descriptor, V4L2_CID_GAIN, parameters.gain).map_err(|source| {
            OpenError::GainNotSet {
                source,
                gain: parameters.gain,
            }
        })?;
        set_control(
            file_descriptor,
            V4L2_CID_HUE_AUTO,
            match parameters.hue_auto {
                true => 1,
                false => 0,
            },
        )
        .map_err(|source| OpenError::HueAutoNotSet {
            source,
            hue_auto: parameters.hue_auto,
        })?;
        set_control(
            file_descriptor,
            V4L2_CID_WHITE_BALANCE_TEMPERATURE,
            parameters.white_balance_temperature,
        )
        .map_err(|source| OpenError::WhiteBalanceTemperatureNotSet {
            source,
            white_balance_temperature: parameters.white_balance_temperature,
        })?;
        set_control(file_descriptor, V4L2_CID_SHARPNESS, parameters.sharpness).map_err(
            |source| OpenError::SharpnessNotSet {
                source,
                sharpness: parameters.sharpness,
            },
        )?;
        set_control(
            file_descriptor,
            V4L2_CID_EXPOSURE_AUTO,
            parameters.exposure_auto as i32,
        )
        .map_err(|source| OpenError::ExposureAutoNotSet {
            source,
            exposure_auto: parameters.exposure_auto,
        })?;
        set_control(
            file_descriptor,
            V4L2_CID_EXPOSURE_ABSOLUTE,
            parameters.exposure_absolute,
        )
        .map_err(|source| OpenError::ExposureAbsoluteNotSet {
            source,
            exposure_absolute: parameters.exposure_absolute,
        })?;
        set_control(
            file_descriptor,
            V4L2_CID_FOCUS_ABSOLUTE,
            parameters.focus_absolute,
        )
        .map_err(|source| OpenError::FocusAbsoluteNotSet {
            source,
            focus_absolute: parameters.focus_absolute,
        })?;
        set_control(
            file_descriptor,
            V4L2_CID_FOCUS_AUTO,
            match parameters.focus_auto {
                true => 1,
                false => 0,
            },
        )
        .map_err(|source| OpenError::FocusAutoNotSet {
            source,
            focus_auto: parameters.focus_auto,
        })?;

        set_automatic_exposure_control_weights(
            file_descriptor,
            parameters.automatic_exposure_control_weights,
        )
        .map_err(|source| OpenError::AutomaticExposureControlWeightsNotSet {
            source,
            weights: parameters.automatic_exposure_control_weights,
        })?;

        if parameters.disable_digital_effects {
            disable_digital_effects(file_descriptor)
                .map_err(|source| OpenError::DigitalEffectsNotDisabled { source })?;
        }

        if parameters.flip_sensor {
            flip_sensor(file_descriptor).map_err(|source| OpenError::NotFlipped { source })?;
        }

        request_user_pointer_buffers(file_descriptor, parameters.amount_of_buffers).map_err(
            |source| OpenError::UserPointerBuffersNotRequested {
                source,
                amount_of_buffers: parameters.amount_of_buffers,
            },
        )?;

        Ok(Self {
            file_descriptor,
            queued_buffers: vec![None; parameters.amount_of_buffers as usize],
            next_queued_buffer_index: 0,
        })
    }

    pub fn start(&self) -> Result<(), StreamingError> {
        stream_on(self.file_descriptor)
    }

    pub fn stop(&mut self) -> Result<Vec<Vec<u8>>, StreamingError> {
        stream_off(self.file_descriptor)?;
        self.next_queued_buffer_index = 0;
        Ok(self
            .queued_buffers
            .iter_mut()
            .filter_map(|slot| slot.take())
            .collect())
    }

    pub fn poll(&self, timeout: Option<Duration>) -> Result<(), PollingError> {
        let mut file_descriptors = [pollfd {
            fd: self.file_descriptor,
            events: POLLIN | POLLPRI,
            revents: 0,
        }];
        let timeout = timeout
            .map(|timeout| timeout.as_millis() as i32)
            .unwrap_or(-1);
        let number_of_events =
            Errno::result(unsafe { poll(&mut file_descriptors as *mut _, 1, timeout) })
                .map_err(|source| PollingError::DeviceNotPolled { source })?;
        match number_of_events {
            0 => Err(PollingError::DevicePollingTimedOut),
            _ => {
                if file_descriptors[0].revents & (POLLIN | POLLPRI) != 0 {
                    Ok(())
                } else {
                    Err(PollingError::DevicePollingReturnedNoEvents {
                        returned_events: file_descriptors[0].revents,
                    })
                }
            }
        }
    }

    pub fn queue(&mut self, buffer: Vec<u8>) -> Result<(), BufferError> {
        let amount_of_buffers = self.queued_buffers.len();
        let slot = &mut self.queued_buffers[self.next_queued_buffer_index];
        if slot.is_some() {
            return Err(BufferError::NoSlotAvailable {
                non_fitting_buffer: buffer,
            });
        }
        *slot = Some(buffer);
        let buffer_index = self.next_queued_buffer_index;
        self.next_queued_buffer_index += 1;
        self.next_queued_buffer_index %= amount_of_buffers;
        queue(
            self.file_descriptor,
            buffer_index as u32,
            slot.as_ref().unwrap(),
        )
        .map_err(|source| BufferError::BufferNotQueued { source })
    }

    pub fn dequeue(&mut self) -> Result<Vec<u8>, BufferError> {
        let buffer_index = dequeue(self.file_descriptor)
            .map_err(|source| BufferError::BufferNotDequeued { source })?;
        if self.queued_buffers[buffer_index as usize].is_none() {
            return Err(BufferError::SlotNotOccupied { buffer_index });
        }
        Ok(self.queued_buffers[buffer_index as usize].take().unwrap())
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        unsafe { close(self.file_descriptor) };
    }
}
