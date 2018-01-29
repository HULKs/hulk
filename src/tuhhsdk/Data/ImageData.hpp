#pragma once

#include <Modules/Debug/Debug.h>
#include "Tools/Storage/Image.hpp"
#include "Framework/DataType.hpp"
#include "Hardware/CameraInterface.hpp"
#include "Tools/Time.hpp"

class ImageData : public DataType<ImageData> {
public:
  /// the camera that took the image
  Camera camera;
  /// a string identifying the camera
  std::string identification;
  /// the pixel data and size
  Image image;
  /// the system time at which the first pixel has been recorded
  TimePoint timestamp;
  /// the number of seconds that had to be waited for the image
  float wait_time;
  /**
   * @brief reset sets the image to a defined state, does nothing at the moment
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    // TODO: May do something here.
  }

  virtual void fromValue(const Uni::Value&)
  {
  }
};
