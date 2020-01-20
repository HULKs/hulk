#include "ImageReceiver.hpp"

#include "print.hpp"

#include <tmmintrin.h>


ImageReceiver::ImageReceiver(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycle_info_(*this)
  , image_data_(*this)
{
  robotInterface().getCamera(Camera::TOP).startCapture();
  robotInterface().getCamera(Camera::BOTTOM).startCapture();
  image_data_->is_provided = true;
}

ImageReceiver::~ImageReceiver()
{
  image_data_->is_provided = false;
  robotInterface().getCamera(Camera::BOTTOM).stopCapture();
  robotInterface().getCamera(Camera::TOP).stopCapture();
}

void ImageReceiver::cycle()
{
  // Get new image
  auto& image422 = image_data_->image422;
  CameraInterface& camera = robotInterface().getNextCamera();
  image_data_->wait_time = camera.waitForImage();
  image_data_->timestamp = camera.readImage(image422);

  // This needs to be the first call to debug in the ModuleManager per cycle
  debug().setUpdateTime(image_data_->timestamp);

  image_data_->camera = camera.getCameraType();
  switch (image_data_->camera)
  {
    case Camera::TOP:
      image_data_->identification = "top";
      break;
    case Camera::BOTTOM:
    default:
      image_data_->identification = "bottom";
      break;
  }

  cycle_info_->cycleTime = 0.01666f;
  cycle_info_->startTime = image_data_->timestamp;
  cycle_info_->valid = true;

  debug().update(mount_ + "." + image_data_->identification + "_wait_time", image_data_->wait_time);
  if (debug().isSubscribed(mount_ + "." + image_data_->identification + "_image"))
  {
    debug().sendImage(mount_ + "." + image_data_->identification + "_image", image_data_->image422.to444Image());
  }
}
