#include "Vision/ImageReceiver/ImageReceiver.hpp"

ImageReceiver::ImageReceiver(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , imageData_(*this)
{
  robotInterface().enableImageDataProducer();
  imageData_->valid = true;
}

ImageReceiver::~ImageReceiver()
{
  imageData_->valid = false;
  robotInterface().disableImageDataProducer();
}

void ImageReceiver::cycle()
{
  robotInterface().produceImageData(*cycleInfo_, *imageData_);

  // This needs to be the first call to debug in the ModuleManager per cycle
  debug().setUpdateTime(imageData_->captureTimePoint);

  if (debug().isSubscribed(mount_ + "." + imageData_->identification + "_image"))
  {
    debug().sendImage(mount_ + "." + imageData_->identification + "_image",
                      imageData_->image422.to444Image());
  }
}
