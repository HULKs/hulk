#include "ImageReceiver.hpp"

ImageReceiver::ImageReceiver(const ModuleManagerInterface& manager)
  : Module(manager, "ImageReceiver")
  , image_data_(*this)
{
  robotInterface().getCamera(Camera::TOP).startCapture();
  robotInterface().getCamera(Camera::BOTTOM).startCapture();
}

ImageReceiver::~ImageReceiver()
{
  robotInterface().getCamera(Camera::TOP).stopCapture();
  robotInterface().getCamera(Camera::BOTTOM).stopCapture();
}

void ImageReceiver::cycle()
{
  CameraInterface& camera = robotInterface().getCurrentCamera();
  image_data_->wait_time = camera.waitForImage();
  image_data_->timestamp = camera.readImage(image_data_->image);
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
  debug().update(mount_ + "." + image_data_->identification + "_wait_time", image_data_->wait_time);
  debug().sendImage(mount_ + "." + image_data_->identification + "_image", image_data_->image);
}
