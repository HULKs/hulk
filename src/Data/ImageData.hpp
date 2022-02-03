#pragma once

#include "Framework/DataType.hpp"
#include "Framework/Debug/Debug.h"
#include "Hardware/Clock.hpp"
#include "Hardware/Definitions.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/Image422.hpp"

class ImageData : public DataType<ImageData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"ImageData"};
  /// the camera that took the image
  CameraPosition cameraPosition;
  /// a string identifying the camera
  std::string identification;
  /// the pixel data and size as 422 image
  Image422 image422;
  /// the system time at which the first pixel has been recorded
  Clock::time_point captureTimePoint;
  /// true if the image data is actually provided
  bool valid{false};
  /**
   * @brief reset sets the image to a defined state, does nothing at the moment
   */
  void reset() override
  {
    // THIS MUST BE EMPTY FOR SECURITY REASONS
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["cameraType"] << static_cast<unsigned int>(cameraPosition);
    value["identification"] << identification;
    value["captureTimePoint"] << captureTimePoint;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value&) override {}
};
